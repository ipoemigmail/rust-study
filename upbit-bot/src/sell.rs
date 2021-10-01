use crate::app::*;
use crate::noti;
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

pub struct SellerServiceSimple<U: upbit::UpbitService, N: noti::NotiSender> {
    upbit_service: Arc<U>,
    noti_sender: Arc<N>,
}

impl<U: upbit::UpbitService, N: noti::NotiSender> SellerServiceSimple<U, N> {
    pub fn new(upbit_service: Arc<U>, noti_sender: Arc<N>) -> SellerServiceSimple<U, N> {
        SellerServiceSimple {
            upbit_service,
            noti_sender,
        }
    }
}

fn moving_average<'a, I: Iterator<Item = &'a Decimal> + Clone>(prices: I) -> f64 {
    let len = prices.clone().count() as f64;
    let sum = prices.map(|x| x.to_f64().unwrap_or(0.0)).sum::<f64>();
    if (len as isize) == 0 {
        0.0
    } else {
        sum / len
    }
}

#[async_trait]
impl<U: upbit::UpbitService, N: noti::NotiSender> SellerService for SellerServiceSimple<U, N> {
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
                                let msg = format!(
                                    "sell {} ({}) -> moving_avg5: {}, moving_avg20: {}",
                                    market_id, ticker.trade_price, moving_avg5, moving_avg20
                                );
                                info!("{}", msg);
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
                                    Ok(_) => {
                                        let now = chrono::Local::now();
                                        self.noti_sender.send_msg(&format!("[{}] {}", now.to_rfc3339(), msg)).await.unwrap_or(())
                                    }
                                    Err(err) => error!("{} ({}:{})", err, file!(), line!()),
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
