use crate::{app::*, upbit};

use itertools::*;
use std::iter::FromIterator;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use async_trait::async_trait;
use rust_decimal::prelude::*;

pub struct UpbitServiceDummyAccount<U: upbit::UpbitService, A: AppStateService> {
    upbit_service: Arc<U>,
    app_state_service: Arc<A>,
}

impl<U: upbit::UpbitService, A: AppStateService> UpbitServiceDummyAccount<U, A> {
    pub fn new(upbit_service: Arc<U>, app_state_service: Arc<A>) -> UpbitServiceDummyAccount<U, A> {
        UpbitServiceDummyAccount {
            upbit_service,
            app_state_service,
        }
    }

    pub async fn new_with_amount(
        amount: i32,
        upbit_service: Arc<U>,
        app_state_service: Arc<A>,
    ) -> UpbitServiceDummyAccount<U, A> {
        let n = Self::new(upbit_service, app_state_service.clone());
        let mut account = upbit::Account::default();
        account.currency = "KRW".to_owned();
        account.balance = Decimal::from_i32(amount).unwrap();
        account.avg_buy_price = Decimal::ZERO;
        account.avg_buy_price_modified = true;
        account.unit_currency = "KRW".to_owned();
        let mut hs = HashMap::new();
        hs.insert(account.currency.clone(), account);
        app_state_service.set_accounts(hs).await;
        n
    }
}

fn add_account(
    market_id: String,
    price: Decimal,
    volume: Decimal,
    accounts: Arc<HashMap<String, upbit::Account>>,
) -> HashMap<String, upbit::Account> {
    apply_account(market_id, 1, price, volume, accounts)
}

fn sub_account(
    market_id: String,
    price: Decimal,
    volume: Decimal,
    accounts: Arc<HashMap<String, upbit::Account>>,
) -> HashMap<String, upbit::Account> {
    let acc = apply_account(market_id, -1, price, volume, accounts);
	HashMap::from_iter(acc.into_iter().filter(|(s, x)| s == "KRW" || x.balance != Decimal::ZERO))
}

fn apply_account(
    market_id: String,
    sign: i8,
    price: Decimal,
    volume: Decimal,
    accounts: Arc<HashMap<String, upbit::Account>>,
) -> HashMap<String, upbit::Account> {
    let mut new_accounts = accounts.as_ref().clone();
    let mut default_account = upbit::Account::default();
    default_account.currency = market_id.replace("KRW-", "");
    default_account.unit_currency = "KRW".to_owned();
    let mut account = accounts
        .get(&default_account.currency)
        .cloned()
        .unwrap_or(default_account);
    let amount = account.avg_buy_price * account.balance + Decimal::from(sign) * price * volume;
    account.balance = account.balance + Decimal::from(sign) * volume;
    account.avg_buy_price = if account.balance == Decimal::ZERO {
		Decimal::ZERO
	} else {
		amount / account.balance
	} ;
    new_accounts.insert(account.currency.clone(), account);
    new_accounts
}

#[async_trait]
impl<U: upbit::UpbitService, A: AppStateService> upbit::UpbitService
    for UpbitServiceDummyAccount<U, A>
{
    async fn market_list(&self) -> Result<Vec<upbit::MarketInfo>, upbit::Error> {
        self.upbit_service.market_list().await
    }

    async fn market_ticker_list(
        &self,
        market_ids: Vec<String>,
    ) -> Result<Vec<upbit::model::Ticker>, upbit::Error> {
        self.upbit_service.market_ticker_list(market_ids).await
    }

    async fn candles_minutes(
        &self,
        unit: upbit::MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<upbit::Candle>, upbit::Error> {
        self.upbit_service
            .candles_minutes(unit, market_id, count)
            .await
    }

    async fn accounts(&self) -> Result<Vec<upbit::Account>, upbit::Error> {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let ret = self
            .app_state_service
            .accounts()
            .await
            .iter()
            .map(|(_, x)| x.clone())
            .collect_vec();
        Ok(ret)
    }

    async fn orders_chance(
        &self,
        market_id: &str,
    ) -> Result<upbit::model::OrderChance, upbit::Error> {
        self.upbit_service.orders_chance(market_id).await
    }

    async fn ticker_stream(
        &self,
        market_ids: &Vec<String>,
    ) -> Result<futures::channel::mpsc::UnboundedReceiver<upbit::TickerWs>, upbit::Error> {
        self.upbit_service.ticker_stream(market_ids).await
    }

    async fn remain_req(&self) -> Arc<HashMap<String, (u32, u32)>> {
        self.upbit_service.remain_req().await
    }

    async fn clear_remain_req(&self) {
        self.upbit_service.clear_remain_req().await
    }

    async fn request_order(
        &self,
        order_req: upbit::OrderRequest,
    ) -> Result<upbit::OrderResponse, upbit::Error> {
        let mut ret = upbit::OrderResponse::default();
        ret.market = order_req.market.clone();
        ret.ord_type = serde_json::to_string(&order_req.order_type).unwrap();
        ret.side = serde_json::to_string(&order_req.side).unwrap();
        ret.trades_count = 1;
        ret.remaining_fee = Decimal::ZERO;
        ret.locked = Decimal::ZERO;
        ret.remaining_fee = Decimal::ZERO;
        ret.remaining_volume = Decimal::ZERO;
        let account = self.app_state_service.accounts().await.get("KRW").cloned();
        if account.is_none() {
            return Err(upbit::Error::InternalError(
                "Account State Error".to_owned(),
            ));
        }
        String::new();
        if account.unwrap().balance < (*BUY_PRICE * (*FEE_FACTOR + Decimal::ONE)) {
            return Err(upbit::Error::InternalError(
                "Insufficient balance".to_owned(),
            ));
        }
        match order_req.order_type {
            upbit::OrderType::Limit => {
                if order_req.price == Decimal::ZERO {
                    Err(upbit::Error::InternalError(
                        "Price must not be 0".to_owned(),
                    ))
                } else if order_req.volume == Decimal::ZERO {
                    Err(upbit::Error::InternalError(
                        "Volume must not be 0".to_owned(),
                    ))
                } else {
                    self.app_state_service
                        .update_accounts(|v: Arc<HashMap<String, upbit::Account>>| {
                            let acc = add_account(
                                order_req.market.clone(),
                                order_req.price,
                                order_req.volume,
                                v,
                            );
                            sub_account(
                                "KRW-KRW".to_owned(),
                                Decimal::ZERO,
                                *BUY_PRICE * (*FEE_FACTOR + Decimal::ONE),
                                Arc::new(acc),
                            )
                        })
                        .await;
                    ret.price = order_req.price;
                    ret.avg_price = order_req.price;
                    ret.executed_volume = order_req.volume;
                    ret.paid_fee = order_req.price * order_req.volume * *FEE_FACTOR;
                    Ok(ret)
                }
            }
            upbit::OrderType::Price => {
                if order_req.side == upbit::OrderSide::Bid && order_req.price != Decimal::ZERO {
                    let ticker_opt = self
                        .app_state_service
                        .last_tick()
                        .await
                        .get(&order_req.market)
                        .cloned();
                    if ticker_opt.is_none() {
                        Err(upbit::Error::InternalError(format!(
                            "Invalid State Error(Not Found {} Last Ticker Info)",
                            order_req.market
                        )))
                    } else {
                        let ticker = ticker_opt.unwrap();
                        let price = ticker.trade_price;
                        let volume = order_req.price / price;
                        self.app_state_service
                            .update_accounts(|v: Arc<HashMap<String, upbit::Account>>| {
                                let acc = add_account(order_req.market.clone(), price, volume, v);
                                sub_account(
                                    "KRW-KRW".to_owned(),
                                    Decimal::ZERO,
                                    *BUY_PRICE * (Decimal::ONE + *FEE_FACTOR),
                                    Arc::new(acc),
                                )
                            })
                            .await;
                        ret.price = price;
                        ret.avg_price = price;
                        ret.executed_volume = order_req.volume;
                        ret.paid_fee = price * volume * *FEE_FACTOR;
                        Ok(ret)
                    }
                } else {
                    Err(upbit::Error::InternalError(format!(
                        "Invalid Request Error({})",
                        serde_json::to_string(&order_req).unwrap()
                    )))
                }
            }
            upbit::OrderType::Market => {
                if order_req.side == upbit::OrderSide::Ask && order_req.volume != Decimal::ZERO {
                    let ticker_opt = self
                        .app_state_service
                        .last_tick()
                        .await
                        .get(&order_req.market)
                        .cloned();
                    if ticker_opt.is_none() {
                        Err(upbit::Error::InternalError(format!(
                            "Invalid State Error(Not Found {} Last Ticker Info)",
                            order_req.market
                        )))
                    } else {
                        let ticker = ticker_opt.unwrap();
                        let price = ticker.trade_price;
                        let volume = order_req.volume;
                        self.app_state_service
                            .update_accounts(|v: Arc<HashMap<String, upbit::Account>>| {
                                let acc = sub_account(order_req.market.clone(), price, volume, v);
                                add_account(
                                    "KRW-KRW".to_owned(),
                                    Decimal::ZERO,
                                    *BUY_PRICE * (Decimal::ONE - *FEE_FACTOR),
                                    Arc::new(acc),
                                )
                            })
                            .await;
                        ret.price = price;
                        ret.avg_price = price;
                        ret.executed_volume = order_req.volume;
                        ret.paid_fee = price * volume * *FEE_FACTOR;
                        Ok(ret)
                    }
                } else {
                    Err(upbit::Error::InternalError(format!(
                        "Invalid Request Error({})",
                        serde_json::to_string(&order_req).unwrap()
                    )))
                }
            }
        }
    }
}