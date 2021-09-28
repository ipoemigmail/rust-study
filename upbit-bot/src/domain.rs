use crate::upbit::{model::TickerWs, Account, Candle, UpbitService};
use async_lock::RwLock;
use async_trait::async_trait;
use format_num::format_num;
use futures::{SinkExt, StreamExt};
use itertools::*;
use rust_decimal::prelude::*;
use static_init::dynamic;
use tracing::info;

use std::{collections::{HashMap, HashSet}, sync::Arc};

pub trait ToLines {
    fn lines(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub is_shutdown: bool,
    pub market_ids: Arc<Vec<String>>,
    pub candles: Arc<HashMap<String, Arc<Vec<Candle>>>>,
    pub last_tick: Arc<HashMap<String, TickerWs>>,
    pub accounts: Arc<HashSet<Account>>,
    pub log_messages: Arc<Vec<String>>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            is_shutdown: false,
            market_ids: Arc::new(vec![]),
            candles: Arc::new(HashMap::new()),
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

        let mut history_result = (*self.candles)
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
            self.candles.iter().count(),
            self.last_tick.iter().count(),
        )];
        history_result.sort();
        result.extend(history_result);
        result.extend(accounts_result);
        result
    }
}

pub trait SendArcF<A>: (FnOnce(Arc<A>) -> A) + Send {}
impl <A, T: (FnOnce(Arc<A>) -> A) + Send> SendArcF<A> for T {}

pub trait SendF<A>: (FnOnce(A) -> A) + Send {}
impl <A, T: (FnOnce(A) -> A) + Send> SendF<A> for T {}

#[async_trait]
pub trait AppStateService: Send + Sync {
    async fn state(&self) -> AppState;
    async fn is_shutdown(&self) -> bool;
    async fn set_shutdown(&self, is_shutdown: bool);
    async fn update_shutdown<F: SendF<bool>>(&self, f: F);
    async fn market_ids(&self) -> Arc<Vec<String>>;
    async fn set_market_ids(&self, market_ids: Vec<String>);
    async fn update_market_ids<F: SendArcF<Vec<String>>>(&self, f: F);
    async fn candles(&self) -> Arc<HashMap<String, Arc<Vec<Candle>>>>;
    async fn set_candles(&self, candles: HashMap<String, Arc<Vec<Candle>>>);
    async fn update_candles<F: SendArcF<HashMap<String, Arc<Vec<Candle>>>>>(&self, f: F);
    async fn last_tick(&self) -> Arc<HashMap<String, TickerWs>>;
    async fn set_last_tick(&self, last_tick: HashMap<String, TickerWs>);
    async fn update_last_tick<F: SendArcF<HashMap<String, TickerWs>>>(&self, f: F);
    async fn accounts(&self) -> Arc<HashSet<Account>>;
    async fn set_accounts(&self, accounts: HashSet<Account>);
    async fn update_accounts<F: SendArcF<HashSet<Account>>>(&self, f: F);
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

    async fn candles(&self) -> Arc<HashMap<String, Arc<Vec<Candle>>>> {
        self.app_state.read().await.candles.clone()
    }

    async fn set_candles(&self, candles: HashMap<String, Arc<Vec<Candle>>>) {
        self.app_state.write().await.candles = Arc::new(candles);
    }

    async fn update_candles<F: SendArcF<HashMap<String, Arc<Vec<Candle>>>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.candles = Arc::new(f(guard.candles.clone()))
    }

    async fn last_tick(&self) -> Arc<HashMap<String, TickerWs>> {
        self.app_state.read().await.last_tick.clone()
    }

    async fn set_last_tick(&self, last_tick: HashMap<String, TickerWs>) {
        self.app_state.write().await.last_tick = Arc::new(last_tick);
    }

    async fn update_last_tick<F: SendArcF<HashMap<String, TickerWs>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.last_tick= Arc::new(f(guard.last_tick.clone()))
    }

    async fn accounts(&self) -> Arc<HashSet<Account>> {
        self.app_state.read().await.accounts.clone()
    }

    async fn set_accounts(&self, accounts: HashSet<Account>) {
        self.app_state.write().await.accounts = Arc::new(accounts);
    }

    async fn update_accounts<F: SendArcF<HashSet<Account>>>(&self, f: F) {
        let mut guard = self.app_state.write().await;
        guard.accounts = Arc::new(f(guard.accounts.clone()))
    }

    async fn log_messages(&self) -> Arc<Vec<String>> {
        self.app_state.read().await.log_messages.clone()
    }

    async fn set_log_messages(&self, log_messages: Vec<String>) {
        self.app_state.write().await.market_ids = Arc::new(log_messages);
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

impl <S: AppStateService + 'static> AppStateWriter<S> {
    pub fn new(app_state_service: Arc<S>) -> AppStateWriter<S> {
        let (tx, mut rx) = futures::channel::mpsc::channel::<AppStateWriterMsg>(512);
        let _app_state_service = app_state_service.clone();
        tokio::spawn(async move {
            loop {
                match rx.next().await {
                    Some(AppStateWriterMsg::Write(msg)) => {
                        _app_state_service.clone().update_log_messages(move |v: Arc<Vec<String>>| {
                            let mut new = v.as_ref().clone();
                            new.insert(
                                0,
                                std::str::from_utf8(msg.as_slice())
                                    .unwrap()
                                    .trim()
                                    .to_owned(),
                            );
                            new
                        }).await;
                    }
                    Some(AppStateWriterMsg::Flush) => {
                        _app_state_service.clone().set_log_messages(vec![]).await;
                    }
                    None => break,
                }
            }
        });
        AppStateWriter { app_state_service, tx }
    }

    pub async fn close(&mut self) -> Result<(), futures::channel::mpsc::SendError> {
        self.tx.close().await
    }
}

impl <S: AppStateService> std::io::Write for AppStateWriter<S> {
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

impl <S: AppStateService + Clone> tracing_subscriber::fmt::MakeWriter for AppStateWriter<S> {
    type Writer = AppStateWriter<S>;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
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
            match app_state.candles.get(market_id) {
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

                        if is_golden_cross && is_abnormal_volume && ticker.trade_price >= *MIN_PRICE
                        {
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
