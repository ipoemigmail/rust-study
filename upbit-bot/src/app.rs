use crate::upbit;

use async_lock::RwLock;
use async_trait::async_trait;
use format_num::format_num;
use futures::{SinkExt, StreamExt};

use itertools::*;
use lazy_static::lazy_static;
use rust_decimal::prelude::*;

use std::{collections::HashMap, sync::Arc};

lazy_static! {
    pub static ref VOLUME_FACTOR: Decimal = Decimal::from_f64(5.0).unwrap();
    pub static ref MIN_PRICE: Decimal = Decimal::from(1_000);
    pub static ref BUY_PRICE: Decimal = Decimal::from(100_000);
    pub static ref FEE_FACTOR: Decimal = Decimal::from_f64(0.0002).unwrap();
}

pub trait ToInfo {
    fn account_info(&self) -> Vec<String>;
    fn candle_info(&self) -> Vec<String>;
    fn state_info(&self) -> Vec<String>;
    fn message_info(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub is_shutdown: bool,
    pub market_ids: Arc<Vec<String>>,
    pub candles: Arc<HashMap<String, Arc<Vec<upbit::Candle>>>>,
    pub last_tick: Arc<HashMap<String, upbit::TickerWs>>,
    pub accounts: Arc<HashMap<String, upbit::Account>>,
    pub log_messages: Arc<Vec<String>>,
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
        }
    }
}

impl ToInfo for AppState {
    fn account_info(&self) -> Vec<String> {
        self.accounts
            .iter()
            .map(|(_, a)| {
                let v = self
                    .last_tick
                    .get(format!("{}-{}", a.unit_currency, a.currency).as_str());
                let price = if a.currency == "KRW" {
                    1.into()
                } else {
                    v.map(|x| x.trade_price).unwrap_or(Decimal::ZERO)
                };
                let values = (|| -> Option<_> {
                    let fmt_cur_amount = format_num!(",.0f", (a.balance * price).to_f64()?);
                    let fmt_buy_amount =
                        format_num!(",.0f", (a.balance * a.avg_buy_price).to_f64()?);
                    let fmt_balance = format_num!(",.4f", a.balance.to_f64()?);
                    let fmt_avg_buy_price = format_num!(",.2f", a.avg_buy_price.to_f64()?);
                    let fmt_price = format_num!(",.2f", price.to_f64()?);
                    Some((
                        fmt_cur_amount,
                        fmt_buy_amount,
                        fmt_balance,
                        fmt_avg_buy_price,
                        fmt_price,
                    ))
                })();
                match values {
                    Some((cur_amount, buy_amount, balance, avg_buy_price, price)) => format!(
                        "{} - Amount:{}({}), Price:{}({}), Qty: {}, Unit: {}",
                        a.currency,
                        cur_amount,
                        buy_amount,
                        price,
                        avg_buy_price,
                        balance,
                        a.unit_currency
                    ),
                    None => "".to_owned(),
                }
            })
            .sorted()
            .collect_vec()
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
        vec![
            format!("market count: {}", self.market_ids.len()),
            format!("candle count: {}", self.candles.len()),
            format!("last_tick count: {}", self.last_tick.len()),
        ]
    }

    fn message_info(&self) -> Vec<String> {
        self.log_messages
            .iter()
            .enumerate()
            .map(|(i, x)| format!("[{}] {}", i, x))
            .collect_vec()
    }
}

pub trait SendArcF<A>: (FnOnce(Arc<A>) -> A) + Send {}
impl<A, T: (FnOnce(Arc<A>) -> A) + Send> SendArcF<A> for T {}

pub trait SendF<A>: (FnOnce(A) -> A) + Send {}
impl<A, T: (FnOnce(A) -> A) + Send> SendF<A> for T {}

#[async_trait]
pub trait AppStateService: Send + Sync {
    async fn state(&self) -> AppState;
    async fn is_shutdown(&self) -> bool;
    async fn set_shutdown(&self, is_shutdown: bool);
    async fn update_shutdown<F: SendF<bool>>(&self, f: F);
    async fn market_ids(&self) -> Arc<Vec<String>>;
    async fn set_market_ids(&self, market_ids: Vec<String>);
    async fn update_market_ids<F: SendArcF<Vec<String>>>(&self, f: F);
    async fn candles(&self) -> Arc<HashMap<String, Arc<Vec<upbit::Candle>>>>;
    async fn set_candles(&self, candles: HashMap<String, Arc<Vec<upbit::Candle>>>);
    async fn update_candles<F: SendArcF<HashMap<String, Arc<Vec<upbit::Candle>>>>>(&self, f: F);
    async fn last_tick(&self) -> Arc<HashMap<String, upbit::TickerWs>>;
    async fn set_last_tick(&self, last_tick: HashMap<String, upbit::TickerWs>);
    async fn update_last_tick<F: SendArcF<HashMap<String, upbit::TickerWs>>>(&self, f: F);
    async fn accounts(&self) -> Arc<HashMap<String, upbit::Account>>;
    async fn set_accounts(&self, accounts: HashMap<String, upbit::Account>);
    async fn update_accounts<F: SendArcF<HashMap<String, upbit::Account>>>(&self, f: F);
    async fn log_messages(&self) -> Arc<Vec<String>>;
    async fn set_log_messages(&self, log_messages: Vec<String>);
    async fn update_log_messages<F: SendArcF<Vec<String>>>(&self, f: F);
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

    async fn update_shutdown<F: SendF<bool>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.is_shutdown = f(guard.is_shutdown)
    }

    async fn market_ids(&self) -> Arc<Vec<String>> {
        self.app_state.read().await.market_ids.clone()
    }

    async fn set_market_ids(&self, market_ids: Vec<String>) {
        self.app_state.write().await.market_ids = Arc::new(market_ids);
    }

    async fn update_market_ids<F: SendArcF<Vec<String>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.market_ids = Arc::new(f(guard.market_ids.clone()))
    }

    async fn candles(&self) -> Arc<HashMap<String, Arc<Vec<upbit::Candle>>>> {
        self.app_state.read().await.candles.clone()
    }

    async fn set_candles(&self, candles: HashMap<String, Arc<Vec<upbit::Candle>>>) {
        self.app_state.write().await.candles = Arc::new(candles);
    }

    async fn update_candles<F: SendArcF<HashMap<String, Arc<Vec<upbit::Candle>>>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.candles = Arc::new(f(guard.candles.clone()))
    }

    async fn last_tick(&self) -> Arc<HashMap<String, upbit::TickerWs>> {
        self.app_state.read().await.last_tick.clone()
    }

    async fn set_last_tick(&self, last_tick: HashMap<String, upbit::TickerWs>) {
        self.app_state.write().await.last_tick = Arc::new(last_tick);
    }

    async fn update_last_tick<F: SendArcF<HashMap<String, upbit::TickerWs>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.last_tick = Arc::new(f(guard.last_tick.clone()))
    }

    async fn accounts(&self) -> Arc<HashMap<String, upbit::Account>> {
        self.app_state.read().await.accounts.clone()
    }

    async fn set_accounts(&self, accounts: HashMap<String, upbit::Account>) {
        self.app_state.write().await.accounts = Arc::new(accounts);
    }

    async fn update_accounts<F: SendArcF<HashMap<String, upbit::Account>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.accounts = Arc::new(f(guard.accounts.clone()))
    }

    async fn log_messages(&self) -> Arc<Vec<String>> {
        self.app_state.read().await.log_messages.clone()
    }

    async fn set_log_messages(&self, log_messages: Vec<String>) {
        self.app_state.write().await.log_messages = Arc::new(log_messages);
    }

    async fn update_log_messages<F: SendArcF<Vec<String>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.log_messages = Arc::new(f(guard.log_messages.clone()))
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
                        _app_state_service
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
                                new
                            })
                            .await;
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
