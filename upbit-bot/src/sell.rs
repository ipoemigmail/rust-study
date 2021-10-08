use crate::app::*;
use crate::noti;
use crate::upbit;
use crate::util::retry;

use async_trait::async_trait;
use format_num::format_num;
use itertools::Itertools;
use lazy_static::lazy_static;
use rust_decimal::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

lazy_static! {
    pub static ref EARN_PERCENTAGE: Decimal = Decimal::from_f64(0.001).unwrap();
}

#[async_trait]
pub trait SellerService: Send + Sync {
    async fn process(&self, app_state: &AppState) -> Vec<String>;
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

fn is_dead_cross(ticker: &upbit::TickerWs, candles: Arc<Vec<upbit::Candle>>) -> bool {
    let mut cur_candles = candles.iter().map(|x| x.trade_price).collect_vec();
    //cur_candles.insert(0, ticker.trade_price);
    let moving_avg5 = moving_average(cur_candles.iter().take(5));
    let moving_avg20 = moving_average(cur_candles.iter().take(20));

    moving_avg5 < moving_avg20
}

async fn sell_account<U: upbit::UpbitService>(
    market_id: String,
    account: &upbit::Account,
    upbit_service: Arc<U>,
) -> Result<upbit::OrderResponse, upbit::Error> {
    retry(1, Duration::from_millis(10), || async {
        let mut req = upbit::OrderRequest::default();
        req.market = market_id.clone();
        req.side = upbit::OrderSide::Ask;
        req.order_type = upbit::OrderType::Market;
        req.volume = account.balance;
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
                    "[{}] sell {}({}), total: {}",
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
impl<U: upbit::UpbitService, N: noti::NotiSender> SellerService for SellerServiceSimple<U, N> {
    async fn process(&self, app_state: &AppState) -> Vec<String> {
        let mut sold_market_ids = vec![];
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
                        let mut is_sell = false;
                        let mut win_or_lose = "win";
                        match account {
                            Some(a) => {
                                if ticker.trade_price < a.avg_buy_price {
                                    win_or_lose = "lose"
                                }
                                if is_dead_cross(ticker, candles.clone()) {
                                    info!(
                                        "sell {} ({} <- {}) by dead_cross ({})",
                                        market_id,
                                        ticker.trade_price.to_i64().unwrap(),
                                        a.avg_buy_price.to_i64().unwrap(),
                                        win_or_lose
                                    );
                                    is_sell = true;
                                }
                            }
                            None => (),
                        }
                        if is_sell {
                            let ret = sell_account(
                                market_id.clone(),
                                account.unwrap(),
                                self.upbit_service.clone(),
                            )
                            .await;
                            match ret {
                                Ok(_) => {
                                    sold_market_ids.push(market_id.clone());
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
        sold_market_ids
    }
}
