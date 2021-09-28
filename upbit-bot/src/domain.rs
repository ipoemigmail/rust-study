use crate::upbit::{model::TickerWs, Account, Candle, UpbitService};
use async_trait::async_trait;
use format_num::format_num;
use itertools::*;
use rust_decimal::prelude::*;
use static_init::dynamic;
use tracing::info;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
            .into_group_map_by(|x| x.to_owned())
            .into_iter()
            .map(|(x, xs)| format!("{} -> {}", x, xs.len()))
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

fn moving_average<'a, I: Iterator<Item = &'a Candle> + Clone>(candles: I) -> f64 {
    let len = candles.clone().count() as f64;
    let sum = candles
        .map(|x| x.trade_price.to_f64().unwrap_or(0.0))
        .sum::<f64>();
    sum / len
}

#[dynamic(lazy)]
static VOLUME_FACTOR: Decimal = Decimal::from_f64(5.0).unwrap();
#[dynamic(lazy)]
static MIN_PRICE: Decimal = Decimal::from(1_000);

#[async_trait]
impl<U: UpbitService> BuyerService for BuyerServiceSimple<U> {
    async fn process(&self, app_state: &AppState) {
        for market_id in app_state.market_ids.iter() {
            match app_state.history.get(market_id) {
                Some(candles) => match app_state.last_tick.get(market_id) {
                    Some(ticker) => {
                        let moving_avg5 = moving_average(candles.iter().take(5));
                        let moving_avg20 = moving_average(candles.iter().take(20));

                        let prev_moving_avg5 = moving_average(candles.iter().skip(1).take(5));
                        let prev_moving_avg20 = moving_average(candles.iter().skip(1).take(20));

                        let is_golden_cross =
                            prev_moving_avg5 <= prev_moving_avg20 && moving_avg5 > moving_avg20;

                        let volume_sum: Decimal = candles
                            .iter()
                            .take(20)
                            .map(|x| Decimal::from(x.candle_acc_trade_volume))
                            .sum();
                        let avg_volume = volume_sum / Decimal::from(20);
                        let is_abnormal_volume =
                            Decimal::from(ticker.trade_volume) > avg_volume * *VOLUME_FACTOR;

                        if is_golden_cross && is_abnormal_volume && ticker.trade_price >= *MIN_PRICE {
                        //if is_abnormal_volume {
                            info!(
                                "{} -> moving_avg5: {}, moving_avg20: {}, avg volumne: {}, cur volume: {}",
                                market_id, moving_avg5, moving_avg20, avg_volume, ticker.trade_volume
                            );
                        }
                    }
                    None => (),
                },
                None => (),
            }
        }
    }
}
