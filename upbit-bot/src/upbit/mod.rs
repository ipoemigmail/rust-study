use async_trait::async_trait;
use futures::{channel::mpsc::UnboundedReceiver, SinkExt, StreamExt};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use hmac::{Hmac, NewMac};
use itertools::Itertools;
use jwt::SignWithKey;
use reqwest::header::{self, HeaderMap};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::{fmt::Display, sync::Arc, time::Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

pub mod model;
pub use model::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    InternalError(String),
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    SerdeError(#[from] serde_json::error::Error),
    #[error("{0}")]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),
}

pub enum MinuteUnit {
    _1,
    _3,
    _5,
    _10,
    _15,
    _30,
    _60,
    _240,
}

impl Display for MinuteUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MinuteUnit::_1 => "1",
            MinuteUnit::_3 => "3",
            MinuteUnit::_5 => "5",
            MinuteUnit::_10 => "10",
            MinuteUnit::_15 => "15",
            MinuteUnit::_30 => "30",
            MinuteUnit::_60 => "60",
            MinuteUnit::_240 => "240",
        })
    }
}

#[async_trait]
pub trait UpbitService: Send + Sync {
    async fn market_list(&self) -> Result<Vec<MarketInfo>, Error>;
    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<Ticker>, Error>;
    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error>;
    async fn accounts(&self, access_key: &str, secret_key: &str) -> Result<Vec<Account>, Error>;
    async fn orders_chance(
        &self,
        access_key: &str,
        secret_key: &str,
        market_id: &str,
    ) -> Result<OrderChance, Error>;
    async fn ticker_stream(
        &self,
        market_ids: &Vec<String>,
    ) -> Result<futures::channel::mpsc::UnboundedReceiver<TickerWs>, Error>;
}

pub struct UpbitServiceSimple {
    client: reqwest::Client,
    default_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    market_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    candle_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
}

fn safe_limit(c: u32) -> u32 {
    (c as f64 * 0.9) as u32
}

impl UpbitServiceSimple {
    pub fn new() -> UpbitServiceSimple {
        UpbitServiceSimple {
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
            default_rate_limiters: Arc::new(create_limiter(safe_limit(10), safe_limit(500))),
            market_rate_limiters: Arc::new(create_limiter(safe_limit(10), safe_limit(600))),
            candle_rate_limiters: Arc::new(create_limiter(safe_limit(10), safe_limit(600))),
        }
    }
}

const BASE_URL: &str = "https://api.upbit.com/v1";
const WS_BASE_URL: &str = "ws://api.upbit.com/websocket/v1";

async fn call_api_response<A>(
    client: &reqwest::Client,
    url: &str,
    header_map: Option<HeaderMap>,
) -> Result<A, Error>
where
    A: DeserializeOwned,
{
    let req = match header_map {
        Some(hm) => client.get(url).headers(hm),
        None => client.get(url),
    }
    .build()
    .unwrap();
    let resp = client.execute(req).await?;
    //Remaining-Req: group=default; min=1799; sec=29
    let info = resp
        .headers()
        .get("Remaining-Req")
        .unwrap()
        .to_str()
        .unwrap()
        .split(";")
        .map(|x| x.trim().to_owned().split("=").last().unwrap().to_owned())
        .collect_vec();

    crate::insert_req_remain(
        &info[0],
        info[1].parse::<u32>().unwrap(),
        info[2].parse::<u32>().unwrap(),
    )
    .await;
    let remaining_req = resp.headers().get("Remaining-Req").cloned();
    let resp_text = resp.text().await?;
    match serde_json::from_str::<'_, A>(resp_text.as_str()) {
        v @ Ok(_) => v.map_err(|x| x.into()),
        Err(e) => Err(Error::InternalError(format!(
            "{} - {}:{:?} ({})",
            e, url, remaining_req, resp_text
        ))),
    }
}

#[async_trait]
impl UpbitService for UpbitServiceSimple {
    async fn market_list(&self) -> Result<Vec<MarketInfo>, Error> {
        for limiter in self.market_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!("{}/market/all?isDetails=true", BASE_URL);
        call_api_response(&self.client, &url, None).await
    }

    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<Ticker>, Error> {
        for limiter in self.market_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!("{}/ticker?markets={}", BASE_URL, market_ids.join(","));
        call_api_response(&self.client, &url, None).await
    }

    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error> {
        for limiter in self.candle_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!(
            "{}/candles/minutes/{}?market={}&count={}",
            BASE_URL, unit, market_id, count
        );
        call_api_response(&self.client, &url, None).await
    }

    async fn accounts(&self, access_key: &str, secret_key: &str) -> Result<Vec<Account>, Error> {
        for limiter in self.default_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!("{}/accounts", BASE_URL);
        let key: Hmac<Sha512> = Hmac::new_from_slice(secret_key.as_bytes()).unwrap();
        let mut claims = BTreeMap::new();
        claims.insert("access_key", access_key.to_owned());
        claims.insert("nonce", Uuid::new_v4().to_string());
        let token_str = format!("Bearer {}", claims.sign_with_key(&key).unwrap());
        let mut header_map = HeaderMap::new();
        header_map.append(header::AUTHORIZATION, token_str.parse().unwrap());
        call_api_response(&self.client, &url, Some(header_map)).await
    }

    async fn orders_chance(
        &self,
        access_key: &str,
        secret_key: &str,
        market_id: &str,
    ) -> Result<OrderChance, Error> {
        for limiter in self.default_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let query_string = format!("market={}", market_id);
        let url = format!("{}/orders/chance?{}", BASE_URL, query_string);
        let query_hash = Sha512::digest(query_string.as_bytes());
        let key: Hmac<Sha512> = Hmac::new_from_slice(secret_key.as_bytes()).unwrap();
        let mut claims = BTreeMap::new();
        claims.insert("access_key", access_key.to_owned());
        claims.insert("nonce", Uuid::new_v4().to_string());
        claims.insert("query_hash", format!("{:x}", query_hash));
        claims.insert("query_hash_alg", "SHA512".to_owned());
        let token_str = format!("Bearer {}", claims.sign_with_key(&key).unwrap());
        let mut header_map = HeaderMap::new();
        header_map.append(header::AUTHORIZATION, token_str.parse().unwrap());
        call_api_response(&self.client, &url, Some(header_map)).await
    }

    async fn ticker_stream(
        &self,
        market_ids: &Vec<String>,
    ) -> Result<UnboundedReceiver<TickerWs>, Error> {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let (ws_stream, _) = connect_async(WS_BASE_URL).await.expect("Failed to connect");
        //println!("WebSocket handshake has been successfully completed");
        let (mut write, mut read) = ws_stream.split();
        let cmd = format!(
            r#"[{{"ticket":"{}"}},{{"type":"ticker","codes":["{}"]}}]"#,
            uuid::Uuid::new_v4().to_owned(),
            market_ids.join("\",\"")
        );

        write.send(Message::text(cmd)).await?;
        tokio::spawn(async move {
            loop {
                match read.next().await.unwrap() {
                    Ok(m) => {
                        let data = m.into_data();
                        match serde_json::from_slice::<'_, TickerWs>(data.as_slice()) {
                            Ok(ticker) => match tx.unbounded_send(ticker) {
                                Ok(_) => (),
                                Err(e) => {
                                    println!("{}", e);
                                    break;
                                }
                            },
                            Err(e) => println!("{}", e),
                        }
                    }
                    Err(e) => {
                        println!("{}", e);
                        break;
                    }
                }
            }
        });
        Ok(rx)
    }
}

pub struct UpbitServiceDummyAccount {
    client: reqwest::Client,
    accounts: Arc<Vec<Account>>,
    default_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    market_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    candle_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
}

impl UpbitServiceDummyAccount {
    pub fn new() -> UpbitServiceDummyAccount {
        UpbitServiceDummyAccount {
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
            accounts: Arc::new(vec![]),
            default_rate_limiters: Arc::new(create_limiter(safe_limit(30), safe_limit(500))),
            market_rate_limiters: Arc::new(create_limiter(safe_limit(10), safe_limit(600))),
            candle_rate_limiters: Arc::new(create_limiter(safe_limit(10), safe_limit(600))),
        }
    }
}

#[async_trait]
impl UpbitService for UpbitServiceDummyAccount {
    async fn market_list(&self) -> Result<Vec<MarketInfo>, Error> {
        for limiter in self.market_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!("{}/market/all?isDetails=true", BASE_URL);
        call_api_response(&self.client, &url, None).await
    }

    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<Ticker>, Error> {
        for limiter in self.market_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!("{}/ticker?markets={}", BASE_URL, market_ids.join(","));
        call_api_response(&self.client, &url, None).await
    }

    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error> {
        for limiter in self.candle_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let url = format!(
            "{}/candles/minutes/{}?market={}&count={}",
            BASE_URL, unit, market_id, count
        );
        call_api_response(&self.client, &url, None).await
    }

    async fn accounts(&self, _access_key: &str, _secret_key: &str) -> Result<Vec<Account>, Error> {
        for limiter in self.default_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        Ok((*self.accounts).clone())
    }

    async fn orders_chance(
        &self,
        _access_key: &str,
        _secret_key: &str,
        market_id: &str,
    ) -> Result<OrderChance, Error> {
        for limiter in self.default_rate_limiters.iter() {
            limiter.until_ready().await;
        }
        let result = OrderChance {
            market: OrderMarket {
                id: market_id.to_owned(),
                ..OrderMarket::default()
            },
            ..OrderChance::default()
        };
        Ok(result)
    }

    async fn ticker_stream(
        &self,
        _market_ids: &Vec<String>,
    ) -> Result<futures::channel::mpsc::UnboundedReceiver<TickerWs>, Error> {
        todo!()
    }
}

fn create_limiter(
    per_second: u32,
    per_minute: u32,
) -> Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
    let second_interval = Duration::from_secs(1).as_nanos() / (per_second as u128);
    vec![
        RateLimiter::direct(
            Quota::with_period(Duration::from_nanos(second_interval as u64)).unwrap(),
        ),
        RateLimiter::direct(Quota::per_minute(NonZeroU32::new(per_minute).unwrap())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_ws() {
        let upbit_service = Arc::new(UpbitServiceSimple::new());
        let market_ids = vec!["KRW-BTC".to_owned()];
        let s = upbit_service.ticker_stream(&market_ids).await.unwrap();
        s.for_each(|x| async move { println!("{:?}", x) }).await;
    }
}