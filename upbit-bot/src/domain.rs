use crate::upbit::{model::TickerWs, Account, Candle, UpbitService};
use async_trait::async_trait;
use format_num::format_num;
use itertools::*;
use rust_decimal::prelude::*;
use tracing::info;

use std::{collections::{HashMap, HashSet}, sync::{Arc}};

pub trait ToLines {
    fn lines(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub is_shutdown: bool,
    pub market_ids: Arc<Vec<String>>,
    pub history: Arc<HashMap<String, Arc<Vec<Candle>>>>,
    pub last_tick: Arc<HashMap<String, TickerWs>>,
    pub accounts: Arc<HashSet<Account>>,
    pub log_messages: Arc<Vec<String>>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            is_shutdown: false,
            market_ids: Arc::new(vec![]),
            history: Arc::new(HashMap::new()),
            last_tick: Arc::new(HashMap::new()),
            accounts: Arc::new(HashSet::new()),
            log_messages: Arc::new(vec![]),
        }
    }
}

impl ToLines for AppState {
    fn lines(&self) -> Vec<String> {
        let mut accounts_result = self
            .accounts
            .iter()
            .map(|a| {
                let v = self
                    .last_tick
                    .get(format!("{}-{}", a.unit_currency, a.currency).as_str());
                let price = if a.currency == "KRW" {
                    1.into()
                } else {
                    v.map(|x| x.trade_price).unwrap_or(Decimal::ZERO)
                };
                let values = (|| -> Option<_> {
                    let cur_amount = format_num!(",.2", (a.balance * price).to_f64()?);
                    let buy_amount = format_num!(",.2", (a.balance * a.avg_buy_price).to_f64()?);
                    let balance = format_num!(",.4", a.balance.to_f64()?);
                    let avg_buy_price = format_num!(",.2", a.avg_buy_price.to_f64()?);
                    Some((cur_amount, buy_amount, balance, avg_buy_price))
                })();
                match values {
                    Some((cur_amount, buy_amount, balance, avg_buy_price)) => format!(
                        "{} - Current Amount: {}, Quantity: {}, Buy Price: {}, Buy Amount: {}",
                        a.currency, cur_amount, balance, avg_buy_price, buy_amount
                    ),
                    None => "".to_owned(),
                }
            })
            .collect_vec();
        accounts_result.sort();

        let mut history_result = (*self.history)
            .clone()
            .into_iter()
            .map(|x| x.1.first().unwrap().candle_date_time_kst.to_owned())
            .sorted()
            .group_by(|x| x.to_owned())
            .into_iter()
            .map(|(x, xs)| format!("{} -> {}", x, xs.count()))
            .collect_vec();

        let mut result = vec![format!(
            "market count: {}, history count: {}, last_tick count: {}",
            self.market_ids.iter().count(),
            self.history.iter().count(),
            self.last_tick.iter().count(),
        )];
        history_result.sort();
        result.extend(history_result);
        result.extend(accounts_result);
        result
    }
}

#[async_trait]
impl ToLines for Vec<String> {
    fn lines(&self) -> Vec<String> {
        self.clone()
    }
}

#[async_trait]
pub trait BuyerService: Send + Sync {
    async fn process(&self, app_state: &AppState);
}

pub struct BuyerServiceSimple<U: UpbitService + 'static> {
    upbit_service: Arc<U>,
}

impl<U: UpbitService> BuyerServiceSimple<U> {
    pub fn new(upbit_service: Arc<U>) -> BuyerServiceSimple<U> {
        BuyerServiceSimple { upbit_service }
    }
}

#[async_trait]
impl<U: UpbitService> BuyerService for BuyerServiceSimple<U> {
    async fn process(&self, app_state: &AppState) {
        for market_id in app_state.market_ids.iter() {
            match app_state.history.get(market_id) {
                Some(candles) => {
                    let volume_sum: Decimal = candles
                        .iter()
                        .map(|x| Decimal::from(x.candle_acc_trade_volume))
                        .sum();
                    let avg_volume = volume_sum / Decimal::from(candles.len());
                    match app_state.last_tick.get(market_id) {
                        Some(ticker) => {
                            if Decimal::from(ticker.trade_volume) > avg_volume * Decimal::from_f64(2.0).unwrap() {
                                info!("abnormal volumne: {} -> avg: {}, cur: {}", market_id, avg_volume, ticker.trade_volume);
                            }
                        }
                        None => (),
                    }
                }
                None => (),
            }
        }
    }
}
