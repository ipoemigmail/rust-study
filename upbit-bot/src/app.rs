use crate::upbit;

use async_lock::RwLock;
use async_trait::async_trait;
use chrono::prelude::*;
use chrono::DateTime;
use format_num::format_num;
use futures::{SinkExt, StreamExt};
use itertools::*;
use lazy_static::lazy_static;
use rust_decimal::prelude::*;
use std::{collections::HashMap, sync::Arc, time::Duration};

lazy_static! {
    pub static ref VOLUME_FACTOR: Decimal = Decimal::from_f64(2.0).unwrap();
    pub static ref MIN_PRICE: Decimal = Decimal::from(1_000);
    pub static ref BUY_PRICE: Decimal = Decimal::from(100_000);
    pub static ref FEE_FACTOR: Decimal = Decimal::from_f64(0.0005).unwrap();
}

pub trait ToInfo {
    fn account_info(&self) -> Vec<String>;
    fn candle_info(&self) -> Vec<String>;
    fn state_info(&self) -> Vec<String>;
    fn message_info(&self) -> Vec<String>;
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    InternalError(String),
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub is_shutdown: bool,
    pub market_ids: Arc<Vec<String>>,
    pub candles: Arc<HashMap<String, Arc<Vec<upbit::Candle>>>>,
    pub last_tick: Arc<HashMap<String, upbit::TickerWs>>,
    pub accounts: Arc<HashMap<String, upbit::Account>>,
    pub log_messages: Arc<Vec<String>>,
    pub last_buy_time: Arc<HashMap<String, i64>>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            is_shutdown: false,
            market_ids: Arc::new(vec![]),
            candles: Arc::new(HashMap::new()),
            last_tick: Arc::new(HashMap::new()),
            accounts: Arc::new(HashMap::new()),
            log_messages: Arc::new(vec![]),
            last_buy_time: Arc::new(HashMap::new()),
        }
    }
}

impl ToInfo for AppState {
    fn account_info(&self) -> Vec<String> {
        #[derive(Debug, Clone, Default)]
        struct AccountInfo {
            currency: String,
            cur_amount: Decimal,
            buy_amount: Decimal,
            balance: Decimal,
            cur_price: Decimal,
            avg_buy_price: Decimal,
        }

        impl ToString for AccountInfo {
            fn to_string(&self) -> String {
                (|| -> Option<String> {
                    let fmt_cur_amount = format_num!(",.0f", self.cur_amount.to_f64()?);
                    let fmt_buy_amount = format_num!(",.0f", self.buy_amount.to_f64()?);
                    let fmt_balance = format_num!(",.4f", self.balance.to_f64()?);
                    let fmt_cur_price = format_num!(",.0f", self.cur_price.to_f64()?);
                    let fmt_avg_buy_price = format_num!(",.0f", self.avg_buy_price.to_f64()?);
                    Some(format!(
                        "{} - Amount: {}({}), Price: {}({}), Qty: {}",
                        self.currency,
                        fmt_cur_amount,
                        fmt_buy_amount,
                        fmt_cur_price,
                        fmt_avg_buy_price,
                        fmt_balance,
                    ))
                })()
                .unwrap_or("".to_owned())
            }
        }

        let to_info = |a: &upbit::Account| -> Option<AccountInfo> {
            let tick = self
                .last_tick
                .get(format!("{}-{}", a.unit_currency, a.currency).as_str());
            let price = if a.currency == "KRW" {
                1.into()
            } else {
                tick.map(|x| x.trade_price).unwrap_or(Decimal::ZERO)
            };
            let mut value = AccountInfo::default();
            value.currency = a.currency.clone();
            value.cur_amount = a.balance * price;
            value.buy_amount = a.balance * a.avg_buy_price;
            value.balance = a.balance;
            value.cur_price = price;
            value.avg_buy_price = a.avg_buy_price;
            Some(value)
        };

        let krw_account = self.accounts.get("KRW").cloned();
        let krw_except_accounts = self
            .accounts
            .as_ref()
            .clone()
            .into_iter()
            .filter(|x| x.0 != "KRW")
            .collect_vec();

        let mut ret = vec![krw_account
            .map(|x| to_info(&x))
            .flatten()
            .map(|x| x.to_string())
            .unwrap_or("".to_owned())];
        if !krw_except_accounts.is_empty() {
            ret.push("-----------".to_owned());
            ret.extend(
                krw_except_accounts
                    .iter()
                    .map(|(_, a)| to_info(a).map(|x| x.to_string()).unwrap_or("".to_owned()))
                    .sorted()
                    .collect_vec(),
            );
        }
        ret.push("-----------".to_owned());
        let total_amount = self
            .accounts
            .iter()
            .map(|x| to_info(x.1).map(|y| y.cur_amount).unwrap_or(Decimal::ZERO))
            .sum::<Decimal>()
            .to_f64()
            .unwrap_or(0.0);
        ret.push(format!(
            "Total - Amount: {}",
            format_num!(",.0f", total_amount)
        ));
        ret
    }

    fn candle_info(&self) -> Vec<String> {
        self.candles
            .iter()
            .map(|x| {
                let v = x.1.first().unwrap();
                (v.candle_date_time_kst.clone(), v.market.clone())
            })
            .into_group_map_by(|(x, _)| x.to_owned())
            .into_iter()
            .map(|(x, xs)| {
                if xs.len() < 5 {
                    format!(
                        "{} -> {} ({})",
                        x,
                        xs.len(),
                        xs.into_iter()
                            .map(|x| x.1.replace("KRW-", ""))
                            .collect_vec()
                            .join("/")
                    )
                } else {
                    format!("{} -> {}", x, xs.len())
                }
            })
            .sorted()
            .collect_vec()
    }

    fn state_info(&self) -> Vec<String> {
        let last_candle_time = self
            .candles
            .iter()
            .flat_map(|x| x.1.first().map(|x| x.candle_date_time_kst.clone()))
            .max()
            .unwrap_or("N/A".to_owned());
        let last_tick_time = self
            .last_tick
            .iter()
            .map(|x| x.1.timestamp)
            .max()
            .map(|x| {
                let d = std::time::UNIX_EPOCH + Duration::from_millis(x as u64);
                let dt = DateTime::<Local>::from(d);
                dt.format("%Y-%m-%dT%H:%M:%S.%3f").to_string()
            })
            .unwrap_or("N/A".to_owned());
        vec![
            format!("market: {}", self.market_ids.len()),
            format!("candle: {} ({})", self.candles.len(), last_candle_time),
            format!("last_tick: {} ({})", self.last_tick.len(), last_tick_time),
        ]
    }

    fn message_info(&self) -> Vec<String> {
        self.log_messages
            .iter()
            .rev()
            .enumerate()
            .rev()
            .map(|(i, x)| format!("[{}] {}", i, x))
            .collect_vec()
    }
}

pub trait SendArcF<A, E: std::error::Error>: (FnOnce(Arc<A>) -> Result<A, E>) + Send {}
impl<A, E: std::error::Error, T: (FnOnce(Arc<A>) -> Result<A, E>) + Send> SendArcF<A, E> for T {}

pub trait SendF<A, E: std::error::Error>: (FnOnce(A) -> Result<A, E>) + Send {}
impl<A, E: std::error::Error, T: (FnOnce(A) -> Result<A, E>) + Send> SendF<A, E> for T {}

#[async_trait]
pub trait AppStateService: Send + Sync {
    async fn state(&self) -> AppState;
    async fn is_shutdown(&self) -> bool;
    async fn set_shutdown(&self, is_shutdown: bool);
    async fn update_shutdown<E: std::error::Error, F: SendF<bool, E>>(&self, f: F)
        -> Result<(), E>;
    async fn market_ids(&self) -> Arc<Vec<String>>;
    async fn set_market_ids(&self, market_ids: Vec<String>);
    async fn update_market_ids<E: std::error::Error, F: SendArcF<Vec<String>, E>>(
        &self,
        f: F,
    ) -> Result<(), E>;
    async fn candles(&self) -> Arc<HashMap<String, Arc<Vec<upbit::Candle>>>>;
    async fn set_candles(&self, candles: HashMap<String, Arc<Vec<upbit::Candle>>>);
    async fn update_candles<
        E: std::error::Error,
        F: SendArcF<HashMap<String, Arc<Vec<upbit::Candle>>>, E>,
    >(
        &self,
        f: F,
    ) -> Result<(), E>;
    async fn last_tick(&self) -> Arc<HashMap<String, upbit::TickerWs>>;
    async fn set_last_tick(&self, last_tick: HashMap<String, upbit::TickerWs>);
    async fn update_last_tick<
        E: std::error::Error,
        F: SendArcF<HashMap<String, upbit::TickerWs>, E>,
    >(
        &self,
        f: F,
    ) -> Result<(), E>;
    async fn accounts(&self) -> Arc<HashMap<String, upbit::Account>>;
    async fn set_accounts(&self, accounts: HashMap<String, upbit::Account>);
    async fn update_accounts<
        E: std::error::Error,
        F: SendArcF<HashMap<String, upbit::Account>, E>,
    >(
        &self,
        f: F,
    ) -> Result<(), E>;
    async fn log_messages(&self) -> Arc<Vec<String>>;
    async fn set_log_messages(&self, log_messages: Vec<String>);
    async fn update_log_messages<E: std::error::Error, F: SendArcF<Vec<String>, E>>(
        &self,
        f: F,
    ) -> Result<(), E>;
    async fn last_buy_time(&self) -> Arc<HashMap<String, i64>>;
    async fn set_last_buy_time(&self, last_buy_time: HashMap<String, i64>);
    async fn update_last_buy_time<E: std::error::Error, F: SendArcF<HashMap<String, i64>, E>>(
        &self,
        f: F,
    ) -> Result<(), E>;
}

#[derive(Clone)]
pub struct AppStateServiceSimple {
    app_state: Arc<RwLock<AppState>>,
}

impl AppStateServiceSimple {
    pub fn new() -> AppStateServiceSimple {
        AppStateServiceSimple {
            app_state: Arc::new(RwLock::new(AppState::new())),
        }
    }
}

#[async_trait]
impl AppStateService for AppStateServiceSimple {
    async fn state(&self) -> AppState {
        self.app_state.read().await.clone()
    }

    async fn is_shutdown(&self) -> bool {
        self.app_state.read().await.is_shutdown
    }

    async fn set_shutdown(&self, is_shutdown: bool) {
        self.app_state.write().await.is_shutdown = is_shutdown;
    }

    async fn update_shutdown<E: std::error::Error, F: SendF<bool, E>>(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.is_shutdown = f(guard.is_shutdown)?)
    }

    async fn market_ids(&self) -> Arc<Vec<String>> {
        self.app_state.read().await.market_ids.clone()
    }

    async fn set_market_ids(&self, market_ids: Vec<String>) {
        self.app_state.write().await.market_ids = Arc::new(market_ids);
    }

    async fn update_market_ids<E: std::error::Error, F: SendArcF<Vec<String>, E>>(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.market_ids = Arc::new(f(guard.market_ids.clone())?))
    }

    async fn candles(&self) -> Arc<HashMap<String, Arc<Vec<upbit::Candle>>>> {
        self.app_state.read().await.candles.clone()
    }

    async fn set_candles(&self, candles: HashMap<String, Arc<Vec<upbit::Candle>>>) {
        self.app_state.write().await.candles = Arc::new(candles);
    }

    async fn update_candles<
        E: std::error::Error,
        F: SendArcF<HashMap<String, Arc<Vec<upbit::Candle>>>, E>,
    >(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.candles = Arc::new(f(guard.candles.clone())?))
    }

    async fn last_tick(&self) -> Arc<HashMap<String, upbit::TickerWs>> {
        self.app_state.read().await.last_tick.clone()
    }

    async fn set_last_tick(&self, last_tick: HashMap<String, upbit::TickerWs>) {
        self.app_state.write().await.last_tick = Arc::new(last_tick);
    }

    async fn update_last_tick<
        E: std::error::Error,
        F: SendArcF<HashMap<String, upbit::TickerWs>, E>,
    >(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.last_tick = Arc::new(f(guard.last_tick.clone())?))
    }

    async fn accounts(&self) -> Arc<HashMap<String, upbit::Account>> {
        self.app_state.read().await.accounts.clone()
    }

    async fn set_accounts(&self, accounts: HashMap<String, upbit::Account>) {
        self.app_state.write().await.accounts = Arc::new(accounts);
    }

    async fn update_accounts<
        E: std::error::Error,
        F: SendArcF<HashMap<String, upbit::Account>, E>,
    >(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.accounts = Arc::new(f(guard.accounts.clone())?))
    }

    async fn log_messages(&self) -> Arc<Vec<String>> {
        self.app_state.read().await.log_messages.clone()
    }

    async fn set_log_messages(&self, log_messages: Vec<String>) {
        self.app_state.write().await.log_messages = Arc::new(log_messages);
    }

    async fn update_log_messages<E: std::error::Error, F: SendArcF<Vec<String>, E>>(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.log_messages = Arc::new(f(guard.log_messages.clone())?))
    }

    async fn last_buy_time(&self) -> Arc<HashMap<String, i64>> {
        self.app_state.read().await.last_buy_time.clone()
    }

    async fn set_last_buy_time(&self, last_buy_time: HashMap<String, i64>) {
        self.app_state.write().await.last_buy_time = Arc::new(last_buy_time);
    }

    async fn update_last_buy_time<E: std::error::Error, F: SendArcF<HashMap<String, i64>, E>>(
        &self,
        f: F,
    ) -> Result<(), E> {
        let mut guard = self.app_state.write().await;
        Ok(guard.last_buy_time = Arc::new(f(guard.last_buy_time.clone())?))
    }
}

#[derive(Clone)]
pub struct AppStateWriter<S: AppStateService> {
    app_state_service: Arc<S>,
    tx: futures::channel::mpsc::Sender<AppStateWriterMsg>,
}

#[derive(Debug, Clone)]
pub enum AppStateWriterMsg {
    Write(Vec<u8>),
    Flush,
}

impl<S: AppStateService + 'static> AppStateWriter<S> {
    pub fn new(app_state_service: Arc<S>) -> AppStateWriter<S> {
        let (tx, mut rx) = futures::channel::mpsc::channel::<AppStateWriterMsg>(512);
        let _app_state_service = app_state_service.clone();
        tokio::spawn(async move {
            loop {
                match rx.next().await {
                    Some(AppStateWriterMsg::Write(msg)) => {
                        let ret = _app_state_service
                            .clone()
                            .update_log_messages(move |v: Arc<Vec<String>>| {
                                let mut new = v.as_ref().clone();
                                new.insert(
                                    0,
                                    std::str::from_utf8(msg.as_slice())
                                        .unwrap()
                                        .trim()
                                        .to_owned(),
                                );
                                Ok(new) as Result<_, Error>
                            })
                            .await;
                        match ret {
                            Ok(_) => (),
                            Err(err) => println!("{}", err),
                        }
                    }
                    Some(AppStateWriterMsg::Flush) => {
                        _app_state_service.clone().set_log_messages(vec![]).await;
                    }
                    None => break,
                }
            }
        });
        AppStateWriter {
            app_state_service,
            tx,
        }
    }

    pub async fn close(&mut self) -> Result<(), futures::channel::mpsc::SendError> {
        self.tx.close().await
    }
}

impl<S: AppStateService> std::io::Write for AppStateWriter<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx
            .try_send(AppStateWriterMsg::Write(buf.to_vec()))
            .map(|_| buf.len())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, ""))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.tx
            .try_send(AppStateWriterMsg::Flush)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, ""))
    }
}

impl<S: AppStateService + Clone> tracing_subscriber::fmt::MakeWriter for AppStateWriter<S> {
    type Writer = AppStateWriter<S>;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}
