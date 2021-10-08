use crate::upbit;
use crate::util::retry;
use crate::{app::*, noti};

use async_trait::async_trait;
use format_num::format_num;
use itertools::Itertools;
use rust_decimal::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref LONG_AVG_COUNT: usize = 20;
    pub static ref SHORT_AVG_COUNT: usize = 5;
}

#[async_trait]
pub trait BuyerService: Send + Sync {
    async fn process(&self, app_state: &AppState) -> Vec<String>;
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

fn is_enough_volume(ticker: &upbit::TickerWs, candles: Arc<Vec<upbit::Candle>>) -> bool {
    let volume_sum: Decimal = candles
        .iter()
        .take(20)
        .map(|x| Decimal::from(x.candle_acc_trade_volume))
        .sum();
    let avg_volume = volume_sum / Decimal::from(20);
    Decimal::from(ticker.trade_volume) > avg_volume * *VOLUME_FACTOR
}

fn is_golden_cross(ticker: &upbit::TickerWs, candles: Arc<Vec<upbit::Candle>>) -> bool {
    let mut cur_candles = candles.iter().map(|x| x.trade_price).collect_vec();
    //cur_candles.insert(0, ticker.trade_price);
    let moving_avg5 = moving_average(cur_candles.iter().take(*SHORT_AVG_COUNT));
    let moving_avg20 = moving_average(cur_candles.iter().take(*LONG_AVG_COUNT));

    let prev_moving_avg5 = moving_average(cur_candles.iter().skip(1).take(*SHORT_AVG_COUNT));
    let prev_moving_avg20 = moving_average(cur_candles.iter().skip(1).take(*LONG_AVG_COUNT));

    prev_moving_avg5 <= prev_moving_avg20 && moving_avg5 > moving_avg20
}

async fn buy_account<U: upbit::UpbitService>(
    market_id: String,
    upbit_service: Arc<U>,
) -> Result<upbit::OrderResponse, upbit::Error> {
    retry(1, Duration::from_millis(10), || async {
        let mut req = upbit::OrderRequest::default();
        req.market = market_id.clone();
        req.side = upbit::OrderSide::Bid;
        req.order_type = upbit::OrderType::Price;
        req.price = *BUY_PRICE;
        upbit_service.request_order(req).await
    })
    .await
}

async fn send_msg<U: upbit::UpbitService, N: noti::NotiSender>(
    ticker: &upbit::TickerWs,
    market_id: String,
    upbit_service: Arc<U>,
    noti_sender: Arc<N>,
) {
    match upbit_service.accounts().await {
        Ok(accounts) => {
            let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let total = accounts
                .iter()
                .map(|x| if x.currency == "KRW" { Decimal::ONE } else { x.avg_buy_price } * x.balance)
                .sum::<Decimal>();
            let total_str = format_num!(",.0f", total.to_f64().unwrap());
            let price_str = format_num!(",.0f", ticker.trade_price.to_f64().unwrap());
            noti_sender
                .send_msg(&format!(
                    "[{}] buy {}({}), total: {}",
                    now_str,
                    market_id.replace("KRW-", ""),
                    price_str,
                    total_str
                ))
                .await
                .unwrap_or(());
        }
        Err(err) => error!("{} ({}:{})", err, file!(), line!()),
    }
}

#[async_trait]
impl<U: upbit::UpbitService, N: noti::NotiSender> BuyerService for BuyerServiceSimple<U, N> {
    async fn process(&self, app_state: &AppState) -> Vec<String> {
        let mut bought_market_ids = vec![];
        if app_state
            .accounts
            .get("KRW")
            .filter(|acc| acc.balance >= (*BUY_PRICE * (*FEE_FACTOR + Decimal::ONE)))
            .is_none()
        {
            return bought_market_ids;
        }

        let now_millis = chrono::Local::now().timestamp_millis();

        for market_id in app_state.market_ids.iter() {
            let last_buy_time = app_state.last_buy_time.get(market_id).cloned();
            if last_buy_time
                .filter(|time| (time + Duration::from_secs(60).as_millis() as i64) > now_millis)
                .is_some()
            {
                continue;
            }
            match app_state.candles.get(market_id) {
                Some(candles) => match app_state.last_tick.get(market_id) {
                    Some(ticker) => {
                        let account = app_state.accounts.get(&ticker.code.replace("KRW-", ""));
                        let mut is_buy = false;
                        match account {
                            None => {
                                if is_golden_cross(ticker, candles.clone())
                                    && is_enough_volume(ticker, candles.clone())
                                    //&& ticker.trade_price >= *MIN_PRICE
                                {
                                    info!(
                                        "buy {} ({}) by golden_cross",
                                        market_id, ticker.trade_price.to_i64().unwrap(),
                                    );
                                    is_buy = true;
                                }
                            }
                            Some(_) => (),
                        }
                        if is_buy {
                            let ret =
                                buy_account(market_id.clone(), self.upbit_service.clone()).await;

                            match ret {
                                Ok(_) => {
                                    bought_market_ids.push(market_id.clone());
                                    send_msg(
                                        ticker,
                                        market_id.clone(),
                                        self.upbit_service.clone(),
                                        self.noti_sender.clone(),
                                    )
                                    .await;
                                }
                                Err(err) => error!("{} ({}:{})", err, file!(), line!()),
                            }
                        }
                    }
                    None => (),
                },
                None => (),
            }
        }
        bought_market_ids
    }
}
