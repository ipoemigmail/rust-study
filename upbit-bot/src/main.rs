mod upbit;

use static_init::dynamic;
use anyhow::Result;
use chrono::Local;
use futures::stream;
use futures::stream::StreamExt;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use upbit::*;

#[dynamic]
static BUFFER_SIZE: usize = 60 * 10;
#[dynamic]
static DETECTED_RATE: Decimal = Decimal::from_f64(0.05).unwrap();

fn check(xs: &Vec<MarketTicker>) {
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

#[tokio::main]
async fn main() -> Result<()> {
    let upbit_service = Arc::new(UpbitServiceLive::new());
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut market_ticker_buffer: HashMap<String, Arc<Mutex<Vec<MarketTicker>>>> = HashMap::new();
    loop {
        interval.tick().await;
        let s = upbit_service.market_list().await?;
        let ids = s
            .into_iter()
            .filter(|x| x.market.starts_with("KRW"))
            .map(|x| x.market)
            .collect::<Vec<_>>();
        let market_tickers = upbit_service.market_ticker_list(ids).await?;
        for market_ticker in market_tickers {
            let key = market_ticker.market.as_str();
            if let None = market_ticker_buffer.get(market_ticker.market.as_str()) {
                market_ticker_buffer.insert(
                    key.to_owned(),
                    Arc::new(Mutex::new(Vec::with_capacity(BUFFER_SIZE.to_be()))),
                );
            }
            let mut b = market_ticker_buffer.get_mut(key).unwrap().lock().unwrap();
            if b.len() > BUFFER_SIZE.to_be() {
                b.pop();
            }
            b.insert(0, market_ticker);
        }
        let tasks: Vec<_> = market_ticker_buffer
            .iter()
            .map(move |(_, v)| {
                let vv = v.clone();
                tokio::task::spawn_blocking(move || check(vv.lock().unwrap().as_ref()))
            })
            .collect();
        stream::iter(tasks)
            .then(|t| async move { t.await })
            .collect::<Vec<_>>()
            .await;
    }
}
