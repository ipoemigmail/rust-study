use crate::app::*;
use crate::upbit;
use crate::util::retry;

use async_trait::async_trait;
use itertools::Itertools;
use rust_decimal::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

#[async_trait]
pub trait SellerService: Send + Sync {
    async fn process(&self, app_state: &AppState);
}

pub struct SellerServiceSimple<U: upbit::UpbitService> {
    upbit_service: Arc<U>,
}

impl<U: upbit::UpbitService> SellerServiceSimple<U> {
    pub fn new(upbit_service: Arc<U>) -> SellerServiceSimple<U> {
        SellerServiceSimple { upbit_service }
    }
}

fn moving_average<'a, I: Iterator<Item = &'a Decimal> + Clone>(prices: I) -> f64 {
    let len = prices.clone().count() as f64;
    let sum = prices.map(|x| x.to_f64().unwrap_or(0.0)).sum::<f64>();
    sum / len
}

#[async_trait]
impl<U: upbit::UpbitService> SellerService for SellerServiceSimple<U> {
    async fn process(&self, app_state: &AppState) {
        for market_id in app_state.market_ids.iter() {
            match app_state.candles.get(market_id) {
                Some(candles) => match app_state.last_tick.get(market_id) {
                    Some(ticker) => {
                        let account = app_state.accounts.get(&ticker.code.replace("KRW-", ""));
                        if account.is_some() {
                            let mut cur_candles =
                                candles.iter().map(|x| x.trade_price).collect_vec();
                            cur_candles.insert(0, ticker.trade_price);
                            let moving_avg5 = moving_average(cur_candles.iter().take(5));
                            let moving_avg20 = moving_average(cur_candles.iter().take(20));

                            let is_dead_cross = moving_avg5 < moving_avg20;

                            if is_dead_cross {
                                info!(
                                    "sell {} -> moving_avg5: {}, moving_avg20: {}",
                                    market_id, moving_avg5, moving_avg20
                                );
                                let ret = retry(1, Duration::from_millis(10), || async {
                                    let mut req = upbit::OrderRequest::default();
                                    req.market = market_id.clone();
                                    req.side = upbit::OrderSide::Ask;
                                    req.order_type = upbit::OrderType::Market;
                                    req.volume = account.unwrap().balance;
                                    self.upbit_service.request_order(req).await
                                })
                                .await;

                                match ret {
                                    Ok(_) => (),
                                    Err(err) => error!("{}", err),
                                }
                            }
                        }
                    }
                    None => (),
                },
                None => (),
            }
        }
    }
}
