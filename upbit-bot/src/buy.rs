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
pub trait BuyerService: Send + Sync {
    async fn process(&self, app_state: &AppState);
}

pub struct BuyerServiceSimple<U: upbit::UpbitService, N: noti::NotiSender> {
    upbit_service: Arc<U>,
    noti_sender: Arc<N>,
}

impl<U: upbit::UpbitService, N: noti::NotiSender> BuyerServiceSimple<U, N> {
    pub fn new(upbit_service: Arc<U>, noti_sender: Arc<N>) -> BuyerServiceSimple<U, N> {
        BuyerServiceSimple {
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
impl<U: upbit::UpbitService, N: noti::NotiSender> BuyerService for BuyerServiceSimple<U, N> {
    async fn process(&self, app_state: &AppState) {
        if app_state
            .accounts
            .get("KRW")
            .filter(|acc| acc.balance >= (*BUY_PRICE * (*FEE_FACTOR + Decimal::ONE)))
            .is_none()
        {
            return;
        }

        for market_id in app_state.market_ids.iter() {
            match app_state.candles.get(market_id) {
                Some(candles) => match app_state.last_tick.get(market_id) {
                    Some(ticker) => {
                        let account = app_state.accounts.get(&ticker.code.replace("KRW-", ""));
                        if account.is_none() {
                            let mut cur_candles =
                                candles.iter().map(|x| x.trade_price).collect_vec();
                            cur_candles.insert(0, ticker.trade_price);
                            let moving_avg5 = moving_average(cur_candles.iter().take(5));
                            let moving_avg20 = moving_average(cur_candles.iter().take(20));

                            let prev_moving_avg5 =
                                moving_average(cur_candles.iter().skip(1).take(5));
                            let prev_moving_avg20 =
                                moving_average(cur_candles.iter().skip(1).take(20));

                            let is_golden_cross =
                                prev_moving_avg5 <= prev_moving_avg20 && moving_avg5 > moving_avg20;

                            //let volume_sum: Decimal = candles
                            //    .iter()
                            //    .take(20)
                            //    .map(|x| Decimal::from(x.candle_acc_trade_volume))
                            //    .sum();
                            //let avg_volume = volume_sum / Decimal::from(20);
                            //let is_abnormal_volume =
                            //    Decimal::from(ticker.trade_volume) > avg_volume * *VOLUME_FACTOR;

                            if is_golden_cross
                                //&& is_abnormal_volume
                                && ticker.trade_price >= *MIN_PRICE
                            {
                                //if is_abnormal_volume {
                                let msg = format!(
                                    "buy {} ({}) -> moving_avg5: {}, moving_avg20: {}",
                                    //market_id, ticker.trade_price, moving_avg5, moving_avg20, avg_volume, ticker.trade_volume
                                    market_id,
                                    ticker.trade_price,
                                    moving_avg5,
                                    moving_avg20
                                );
                                info!("{}", msg);
                                let ret = retry(1, Duration::from_millis(10), || async {
                                    let mut req = upbit::OrderRequest::default();
                                    req.market = market_id.clone();
                                    req.side = upbit::OrderSide::Bid;
                                    req.order_type = upbit::OrderType::Price;
                                    req.price = *BUY_PRICE;
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
