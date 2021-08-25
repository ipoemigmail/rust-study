//#![deny(warnings)]
mod ui;
mod upbit;

use anyhow::Result;
use async_trait::async_trait;

//use chrono::Local;
use futures::channel::mpsc::{channel, Receiver};
use futures::SinkExt;
use futures::lock::Mutex;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};

//use static_init::dynamic;
use futures::StreamExt;
use std::num::NonZeroU32;
use std::{collections::HashMap, sync::Arc};


use upbit::*;

struct UpbitRateLimiterService<U: UpbitService> {
    order_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    exchange_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    quotation_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    upbit_service: Arc<U>,
}

impl<U: UpbitService> UpbitRateLimiterService<U> {
    fn new(
        upbit_service: Arc<U>,
        order_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
        exchange_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
        quotation_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    ) -> UpbitRateLimiterService<U> {
        UpbitRateLimiterService {
            order_rate_limiters,
            exchange_rate_limiters,
            quotation_rate_limiters,
            upbit_service,
        }
    }
}

#[async_trait]
impl<U: UpbitService> UpbitService for UpbitRateLimiterService<U> {
    async fn market_list(&self) -> Result<Vec<Market>, Error> {
        for rate_limiter in self.quotation_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service.market_list().await
    }

    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<TradeTick>, Error> {
        for rate_limiter in self.quotation_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service.market_ticker_list(market_ids).await
    }

    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error> {
        for rate_limiter in self.quotation_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service
            .candles_minutes(unit, market_id, count)
            .await
    }

    async fn accounts(&self, access_key: &str, secret_key: &str) -> Result<Vec<Account>, Error> {
        for rate_limiter in self.exchange_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service.accounts(access_key, secret_key).await
    }

    async fn orders_chance(
        &self,
        access_key: &str,
        secret_key: &str,
        market_id: &str,
    ) -> Result<OrderChance, Error> {
        for rate_limiter in self.exchange_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service
            .orders_chance(access_key, secret_key, market_id)
            .await
    }
}

/*
#[dynamic]
static BUFFER_SIZE: usize = 60 * 10;
#[dynamic]
static DETECTED_RATE: Decimal = Decimal::from_f64(0.05).unwrap();

fn check(xs: &Vec<TradeTick>) {
    if xs.len() >= *BUFFER_SIZE {
        let mut ys = xs.clone();
        ys.sort_by(|x, y| x.trade_price.partial_cmp(&y.trade_price).unwrap());
        let last = xs.last().unwrap();
        let min = xs.first().unwrap();
        let diff = last.trade_price.clone() - min.trade_price.clone();
        if last.trade_price.clone() * *DETECTED_RATE < diff {
            let dt = Local::now();
            println!(
                "[{}] {}, last: {}, min: {}",
                dt.to_string(),
                last.market,
                last.trade_price,
                min.trade_price
            );
        }
    }
}
*/

#[allow(dead_code)]
struct Wallet {
    pub buy_items: Arc<Vec<Account>>,
}

#[allow(dead_code)]
struct MarketTickerHistory {
    market_tickers: Arc<HashMap<String, Arc<Vec<TradeTick>>>>,
}

#[allow(dead_code)]
async fn create_ticker_stream<U: UpbitService + 'static>(
    upbit_service: Arc<U>,
) -> Receiver<Arc<Vec<TradeTick>>> {
    let (mut a, b) = channel(0);
    tokio::spawn(async move {
        loop {
            match upbit_service.market_list().await {
                Ok(markets) => {
                    let filtered_markets = markets
                        .into_iter()
                        .filter(|x| x.market.starts_with("KRW"))
                        .map(|x| x.market)
                        .collect::<Vec<_>>();
                    match upbit_service.market_ticker_list(filtered_markets).await {
                        Ok(market_tickers) => a.send(Arc::new(market_tickers)).await.unwrap(),
                        Err(e) => println!("{:?}", e),
                    }
                }
                Err(e) => println!("{:?}", e),
            }
        }
    });
    b
}

fn create_limiter(
    per_second: u32,
    per_minute: u32,
) -> Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
    vec![
        RateLimiter::direct(Quota::per_second(NonZeroU32::new(per_second).unwrap())),
        RateLimiter::direct(Quota::per_minute(NonZeroU32::new(per_minute).unwrap())),
    ]
}

#[tokio::main]
async fn main() -> Result<()> {
    let order_rate_limiters = create_limiter(8, 200);
    let exchange_rate_limiters = create_limiter(30, 900);
    let quotation_rate_limiters = create_limiter(10, 600);
    //let upbit_service = Arc::new(UpbitRateLimiterService::new(
    //    Arc::new(UpbitServiceDummyAccount::new()),
    //    Arc::new(order_rate_limiters),
    //    Arc::new(exchange_rate_limiters),
    //    Arc::new(quotation_rate_limiters),
    //));
    let _upbit_service = Arc::new(UpbitRateLimiterService::new(
        Arc::new(UpbitServiceSimple::new()),
        Arc::new(order_rate_limiters),
        Arc::new(exchange_rate_limiters),
        Arc::new(quotation_rate_limiters),
    ));
    let _access_key = "nJYLpyEglbwNGd2DHIjJ1rBCuchEtnL2PXjIdKRO";
    let _secret_key = "E7Fg5LexgdfmXwLYtxk7P7r3L4FzsfkZkdNhTyw5";
    //let r = upbit_service
    //    .orders_chance(access_key, secret_key, "KRW-BTC")
    //    .await;
    //println!("{}", serde_json::to_string(&r.unwrap()).unwrap());
    //let mut s = create_ticker_stream(upbit_service.clone()).await;
    //while let Some(list) = s.next().await {
    //    for t in list.iter() {
    //        println!("{}", chrono::Local::now().to_rfc3339());
    //    }
    //}
    //loop {
    //    let result = upbit_service.market_list().await;
    //    match result {
    //        Ok(v) => v.iter().for_each(|x| println!("{:?}", x)),
    //        Err(e) => println!("{:?}", e),
    //    }
    //}
    //let mut interval = tokio::time::interval(Duration::from_secs(1));
    //let mut market_ticker_buffer: HashMap<String, Arc<Mutex<Vec<MarketTicker>>>> = HashMap::new();
    //loop {
    //    interval.tick().await;
    //    let s = upbit_service.market_list().await?;
    //    let ids = s
    //        .into_iter()
    //        .filter(|x| x.market.starts_with("KRW"))
    //        .map(|x| x.market)
    //        .collect::<Vec<_>>();
    //    let market_tickers = upbit_service.market_ticker_list(ids).await?;
    //    for market_ticker in market_tickers {
    //        let key = market_ticker.market.as_str();
    //        if let None = market_ticker_buffer.get(market_ticker.market.as_str()) {
    //            market_ticker_buffer.insert(
    //                key.to_owned(),
    //                Arc::new(Mutex::new(Vec::with_capacity(BUFFER_SIZE.to_be()))),
    //            );
    //        }
    //        let mut b = market_ticker_buffer.get_mut(key).unwrap().lock().unwrap();
    //        if b.len() > BUFFER_SIZE.to_be() {
    //            b.pop();
    //        }
    //        b.insert(0, market_ticker);
    //    }
    //    let tasks: Vec<_> = market_ticker_buffer
    //        .iter()
    //        .map(move |(_, v)| {
    //            let vv = v.clone();
    //            tokio::task::spawn_blocking(move || check(vv.lock().unwrap().as_ref()))
    //        })
    //        .collect();
    //    stream::iter(tasks)
    //        .then(|t| async move { t.await })
    //        .collect::<Vec<_>>()
    //        .await;
    //}
    ui::run(async_lock::RwLock::new(vec![])).await?;
    Ok(())
}
