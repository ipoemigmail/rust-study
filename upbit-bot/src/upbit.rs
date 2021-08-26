use std::{fmt::Display, sync::Arc, time::Duration};

use async_trait::async_trait;
use format_num::NumberFormat;
use hmac::{Hmac, NewMac};
use jwt::SignWithKey;
use reqwest::header::{self, HeaderMap};
use rust_decimal::prelude::*;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Market {
    #[serde(rename = "market")]
    pub market: String,
    #[serde(rename = "korean_name")]
    pub korean_name: String,
    #[serde(rename = "english_name")]
    pub english_name: String,
    #[serde(rename = "market_warning")]
    pub market_warning: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct TradeTick {
    #[serde(rename = "market")]
    pub market: String,
    #[serde(rename = "trade_date")]
    pub trade_date: String,
    #[serde(rename = "trade_time")]
    pub trade_time: String,
    #[serde(rename = "trade_date_kst")]
    pub trade_date_kst: String,
    #[serde(rename = "trade_time_kst")]
    pub trade_time_kst: String,
    #[serde(rename = "trade_timestamp")]
    pub trade_timestamp: i64,
    #[serde(rename = "opening_price")]
    pub opening_price: Decimal,
    #[serde(rename = "high_price")]
    pub high_price: Decimal,
    #[serde(rename = "low_price")]
    pub low_price: Decimal,
    #[serde(rename = "trade_price")]
    pub trade_price: Decimal,
    #[serde(rename = "prev_closing_price")]
    pub prev_closing_price: Decimal,
    #[serde(rename = "change")]
    pub change: String,
    #[serde(rename = "change_price")]
    pub change_price: Decimal,
    #[serde(rename = "change_rate")]
    pub change_rate: f64,
    #[serde(rename = "signed_change_price")]
    pub signed_change_price: Decimal,
    #[serde(rename = "signed_change_rate")]
    pub signed_change_rate: f64,
    #[serde(rename = "trade_volume")]
    pub trade_volume: Decimal,
    #[serde(rename = "acc_trade_price")]
    pub acc_trade_price: Decimal,
    #[serde(rename = "acc_trade_price_24h")]
    pub acc_trade_price24_h: Decimal,
    #[serde(rename = "acc_trade_volume")]
    pub acc_trade_volume: Decimal,
    #[serde(rename = "acc_trade_volume_24h")]
    pub acc_trade_volume24_h: Decimal,
    #[serde(rename = "highest_52_week_price")]
    pub highest52_week_price: Decimal,
    #[serde(rename = "highest_52_week_date")]
    pub highest52_week_date: String,
    #[serde(rename = "lowest_52_week_price")]
    pub lowest52_week_price: Decimal,
    #[serde(rename = "lowest_52_week_date")]
    pub lowest52_week_date: String,
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Account {
    #[serde(rename = "currency")]
    pub currency: String,
    #[serde(rename = "balance")]
    pub balance: Decimal,
    #[serde(rename = "locked")]
    pub locked: Decimal,
    #[serde(rename = "avg_buy_price")]
    pub avg_buy_price: Decimal,
    #[serde(rename = "avg_buy_price_modified")]
    pub avg_buy_price_modified: bool,
    #[serde(rename = "unit_currency")]
    pub unit_currency: String,
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num = NumberFormat::new();
        let fmtstr = ",.5";
        let balance = num.format(fmtstr, self.balance.to_f64().unwrap());
        let avg_buy_price = num.format(fmtstr, self.avg_buy_price.to_f64().unwrap());
        f.write_fmt(format_args!(
            "{}: Balance - {}, Locked - {}, AverageBuyPrice - {}, UnitCurrency -  {}",
            self.currency, balance, self.locked, avg_buy_price, self.unit_currency
        ))
    }
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Candle {
    pub market: String,
    #[serde(rename = "candle_date_time_utc")]
    pub candle_date_time_utc: String,
    #[serde(rename = "candle_date_time_kst")]
    pub candle_date_time_kst: String,
    #[serde(rename = "opening_price")]
    pub opening_price: Decimal,
    #[serde(rename = "high_price")]
    pub high_price: Decimal,
    #[serde(rename = "low_price")]
    pub low_price: Decimal,
    #[serde(rename = "trade_price")]
    pub trade_price: Decimal,
    pub timestamp: i64,
    #[serde(rename = "candle_acc_trade_price")]
    pub candle_acc_trade_price: Decimal,
    #[serde(rename = "candle_acc_trade_volume")]
    pub candle_acc_trade_volume: Decimal,
    pub unit: u8,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderChance {
    #[serde(rename = "bid_fee")]
    pub bid_fee: Decimal,
    #[serde(rename = "ask_fee")]
    pub ask_fee: Decimal,
    #[serde(rename = "market")]
    pub market: OrderMarket,
    #[serde(rename = "bid_account")]
    pub bid_account: Account,
    #[serde(rename = "ask_account")]
    pub ask_account: Account,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderMarket {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order_types")]
    pub order_types: Arc<Vec<String>>,
    #[serde(rename = "order_sides")]
    pub order_sides: Arc<Vec<String>>,
    #[serde(rename = "bid")]
    pub bid: OrderMin,
    #[serde(rename = "ask")]
    pub ask: OrderMin,
    #[serde(rename = "max_total")]
    pub max_total: Decimal,
    #[serde(rename = "state")]
    pub state: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderMin {
    #[serde(rename = "currency")]
    pub currency: String,
    #[serde(rename = "price_unit")]
    pub price_unit: Option<String>,
    #[serde(rename = "min_total")]
    pub min_total: Decimal,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    SerdeError(#[from] serde_json::error::Error),
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
    async fn market_list(&self) -> Result<Vec<Market>, Error>;
    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<TradeTick>, Error>;
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
}

pub struct UpbitServiceSimple {
    client: reqwest::Client,
}

impl UpbitServiceSimple {
    pub fn new() -> UpbitServiceSimple {
        UpbitServiceSimple {
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
        }
    }
}

pub struct UpbitServiceDummyAccount {
    client: reqwest::Client,
    accounts: Arc<Vec<Account>>,
}

impl UpbitServiceDummyAccount {
    pub fn new() -> UpbitServiceDummyAccount {
        UpbitServiceDummyAccount {
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
            accounts: Arc::new(vec![]),
        }
    }
}

const BASE_URL: &str = "https://api.upbit.com/v1";

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
    let resp_text = resp.text().await?;
    let result = serde_json::from_str(resp_text.as_str());
    match result {
        Ok(v) => Ok(v),
        Err(e) => {
            println!("{:?}", resp_text);
            Err(e.into())
        }
    }
}

#[async_trait]
impl UpbitService for UpbitServiceSimple {
    async fn market_list(&self) -> Result<Vec<Market>, Error> {
        let url = format!("{}/market/all?isDetails=true", BASE_URL);
        call_api_response(&self.client, &url, None).await
    }

    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<TradeTick>, Error> {
        let url = format!("{}/ticker?markets={}", BASE_URL, market_ids.join(","));
        call_api_response(&self.client, &url, None).await
    }

    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error> {
        let url = format!(
            "{}/candles/minutes/{}?market={}&count={}",
            BASE_URL, unit, market_id, count
        );
        call_api_response(&self.client, &url, None).await
    }

    async fn accounts(&self, access_key: &str, secret_key: &str) -> Result<Vec<Account>, Error> {
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
}

#[async_trait]
impl UpbitService for UpbitServiceDummyAccount {
    async fn market_list(&self) -> Result<Vec<Market>, Error> {
        let url = format!("{}/market/all?isDetails=true", BASE_URL);
        call_api_response(&self.client, &url, None).await
    }

    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<TradeTick>, Error> {
        let url = format!("{}/ticker?markets={}", BASE_URL, market_ids.join(","));
        call_api_response(&self.client, &url, None).await
    }

    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error> {
        let url = format!(
            "{}/candles/minutes/{}?market={}&count={}",
            BASE_URL, unit, market_id, count
        );
        call_api_response(&self.client, &url, None).await
    }

    async fn accounts(&self, _access_key: &str, _secret_key: &str) -> Result<Vec<Account>, Error> {
        Ok((*self.accounts).clone())
    }

    async fn orders_chance(
        &self,
        _access_key: &str,
        _secret_key: &str,
        market_id: &str,
    ) -> Result<OrderChance, Error> {
        let result = OrderChance {
            market: OrderMarket {
                id: market_id.to_owned(),
                ..OrderMarket::default()
            },
            ..OrderChance::default()
        };
        Ok(result)
    }
}
