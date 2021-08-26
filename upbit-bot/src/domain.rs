use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Result};


use crate::upbit::{Account, Candle, TradeTick, UpbitService};

pub struct AppState {
    pub history: Arc<HashMap<String, Arc<Vec<Candle>>>>,
    pub last_tick: Arc<Vec<TradeTick>>,
    pub accounts: Arc<HashSet<Account>>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            history: Arc::new(HashMap::new()),
            last_tick: Arc::new(vec![]),
            accounts: Arc::new(HashSet::new()),
        }
    }
}

pub async fn get_all_tickers<U: UpbitService + 'static>(
    upbit_service: Arc<U>,
) -> Result<Vec<TradeTick>> {
    let market_list = upbit_service.market_list().await;
    if let Err(ref e) = market_list {
        println!("{:?}", e);
    }
    let filtered_markets = market_list?
        .into_iter()
        .filter(|x| x.market.starts_with("KRW"))
        .map(|x| x.market)
        .collect::<Vec<_>>();
    let ticker_list = upbit_service.market_ticker_list(filtered_markets).await;
    if let Err(ref e) = ticker_list {
        println!("{:?}", e);
    }
    Ok(ticker_list?)
}
