//#![deny(warnings)]
mod domain;
mod ui;
mod upbit;

use crate::domain::*;
use anyhow::Result;
use async_lock::RwLock;
use chrono::prelude::*;
use chrono::DurationRound;
use dotenv::dotenv;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedReceiver;
use futures::StreamExt;
use futures::future;
use itertools::*;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::iter::FromIterator;
use tracing::error;

use domain::ToLines;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use upbit::*;

#[derive(Clone)]
struct AppStateWriter {
    app_state: Arc<RwLock<AppState>>,
    tx: futures::channel::mpsc::Sender<AppStateWriterMsg>,
    writer: Arc<JoinHandle<()>>,
}

enum AppStateWriterMsg {
    Write(Vec<u8>),
    Flush,
}

impl AppStateWriter {
    fn new(app_state: Arc<RwLock<AppState>>) -> AppStateWriter {
        let (tx, mut rx) = futures::channel::mpsc::channel::<AppStateWriterMsg>(512);
        let _app_state = app_state.clone();
        let writer = tokio::spawn(async move {
            loop {
                match rx.next().await {
                    Some(AppStateWriterMsg::Write(msg)) => {
                        let mut app_state_guard = _app_state.write().await;
                        let mut new_log_messages = app_state_guard.log_messages.as_ref().clone();
                        new_log_messages.insert(
                            0,
                            std::str::from_utf8(msg.as_slice())
                                .unwrap()
                                .trim()
                                .to_owned(),
                        );
                        app_state_guard.log_messages = Arc::new(new_log_messages);
                    }
                    Some(AppStateWriterMsg::Flush) => {
                        let mut app_state_guard = _app_state.write().await;
                        app_state_guard.log_messages = Arc::new(vec![]);
                    }
                    None => break,
                }
            }
        });
        AppStateWriter {
            app_state,
            tx,
            writer: Arc::new(writer),
        }
    }

    pub async fn close(&mut self) -> Result<(), futures::channel::mpsc::SendError> {
        self.tx.close().await
    }
}

impl std::io::Write for AppStateWriter {
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

impl tracing_subscriber::fmt::MakeWriter for AppStateWriter {
    type Writer = AppStateWriter;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let access_key = std::env::var("ACCESS_KEY")
        .map_err(|_| anyhow::format_err!("ACCESS_KEY was not found in environment var"))?;
    let secret_key = std::env::var("SECRET_KEY")
        .map_err(|_| anyhow::format_err!("SECRET_KEY was not found in environment var"))?;

    //let upbit_service = Arc::new(UpbitServiceSimple::new(&access_key, &secret_key));
    let upbit_service = Arc::new(UpbitServiceDummyAccount::new_with_amount(
        &access_key,
        &secret_key,
        1_000_000,
    ));
    let buyer_service = Arc::new(BuyerServiceSimple::new(upbit_service.clone()));

    let app_state = Arc::new(RwLock::new(AppState {
        accounts: Arc::new(HashSet::from_iter(vec![Account {
            currency: "KRW-BTC".to_owned(),
            balance: Decimal::from(1),
            ..Account::default()
        }])),
        ..AppState::new()
    }));
    tracing_subscriber::fmt()
        .with_writer(AppStateWriter::new(app_state.clone()))
        .with_ansi(false)
        .init();
    //let a = upbit_service.accounts(&access_key, &secret_key).await.unwrap();
    //info!("{}", serde_json::to_string(&a).unwrap());

    let mut terminal = ui::create_terminal()?;
    ui::start_ui(&mut terminal)?;

    let tick_rate = Duration::from_millis(25);
    let mut rx = ui::start_ui_ticker(tick_rate);
    let mut ui_state = ui::UiState::new();

    let candle_updater = candle_updater(app_state.clone(), upbit_service.clone()).await;

    let market_list_updater = market_list_updater(app_state.clone(), upbit_service.clone()).await;

    let account_updater = account_updater(app_state.clone(), upbit_service.clone()).await;

    let (ticker_updater, mut ticker_stream) =
        ticker_updater(app_state.clone(), upbit_service.clone()).await;

    let _app_state = app_state.clone();
    let buyer_handle = tokio::spawn(async move {
        loop {
            match ticker_stream.next().await {
                Some(_) => {
                    let __app_state = _app_state.read().await.clone();
                    buyer_service.process(&__app_state).await;
                }
                None => (),
            }
        }
    });

    // cli start
    loop {
        {
            ui_state.main_messages = Arc::new(app_state.read().await.lines());
            ui_state.req_remains = upbit_service.remain_req().await;
            ui_state.debug_messages = app_state.read().await.log_messages.clone();
        }
        match rx.next().await {
            Some(ui::Event::UiEvent(crossterm::event::Event::Key(key_event)))
                if key_event.code == crossterm::event::KeyCode::Char('l')
                    && (key_event
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)) =>
            {
                terminal.clear()?;
                ui_state.debug_messages = Arc::new(vec![]);
                upbit_service.clear_remain_req().await;
            }
            Some(event) => {
                ui_state = ui::handle_input(ui_state, event, &mut terminal).await?;
                app_state.write().await.is_shutdown = ui_state.is_shutdown;
                if ui_state.is_shutdown {
                    break;
                }
            }
            None => break,
        }
    }
    let is_shutdown = app_state.read().await.is_shutdown;
    candle_updater.abort();
    market_list_updater.abort();
    account_updater.abort();
    ticker_updater.abort();
    buyer_handle.abort();
    println!("is_shutdown: {}", is_shutdown);
    Ok(())
}

async fn market_list_updater<U: UpbitService + 'static>(
    app_state: Arc<RwLock<AppState>>,
    upbit_service: Arc<U>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match upbit_service.market_list().await {
                Ok(xs) => {
                    let market_ids = xs
                        .into_iter()
                        .map(|x| x.market)
                        .filter(|x| x.starts_with("KRW"))
                        .collect();
                    app_state.clone().write().await.market_ids = Arc::new(market_ids);
                }
                Err(e) => {
                    error!("{}", e)
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}

async fn candle_updater<U: UpbitService + 'static>(
    app_state: Arc<RwLock<AppState>>,
    upbit_service: Arc<U>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let market_ids = app_state
                .read()
                .await
                .market_ids
                .iter()
                .map(|x| x.clone())
                .collect_vec();

            for market_id in market_ids.iter() {
                match upbit_service
                    .candles_minutes(MinuteUnit::_1, market_id, 200)
                    .await
                {
                    Ok(xs) => {
                        let mut new_history = (&*app_state.read().await.history).clone();
                        new_history.insert(market_id.to_owned(), Arc::new(xs));
                        app_state.write().await.history = Arc::new(new_history);
                    }
                    Err(e) => {
                        error!("{}", e)
                    }
                }
            }

            if market_ids.len() > 0 {
                let now = Local::now();
                let now_instant = tokio::time::Instant::now();

                let next_time = now.duration_trunc(chrono::Duration::minutes(1)).unwrap()
                    + chrono::Duration::minutes(1);
                let next_instant = now_instant
                    + tokio::time::Duration::from_millis(
                        (next_time.timestamp_millis() - now.timestamp_millis()) as u64,
                    );

                tokio::time::sleep_until(next_instant).await;
            }
        }
    })
}

async fn account_updater<U: UpbitService + 'static>(
    app_state: Arc<RwLock<AppState>>,
    upbit_service: Arc<U>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let accounts = upbit_service.accounts().await;
            match accounts {
                Ok(xs) => {
                    app_state.clone().write().await.accounts = Arc::new(HashSet::from_iter(xs))
                }
                Err(e) => {
                    error!("{}", e)
                }
            }
        }
    })
}

async fn ticker_updater<U: UpbitService + 'static>(
    app_state: Arc<RwLock<AppState>>,
    upbit_service: Arc<U>,
) -> (JoinHandle<()>, UnboundedReceiver<TickerWs>) {
    let (tx, rx) = mpsc::unbounded();
    let handle = tokio::spawn(async move {
        loop {
            let market_ids = app_state
                .read()
                .await
                .market_ids
                .iter()
                .map(|x| x.clone())
                .collect_vec();

            if market_ids.len() > 0 {
                let mut stream = upbit_service
                    .clone()
                    .ticker_stream(&market_ids)
                    .await
                    .unwrap();

                loop {
                    match stream.next().await {
                        Some(ticker) => {
                            {
                                let mut app_state1 = app_state.write().await;
                                let mut new_last_tick = app_state1.last_tick.as_ref().clone();
                                new_last_tick.insert(ticker.code.clone(), ticker.clone());
                                //let v = new_last_tick.keys().map(|x| x.clone()).collect::<Vec<_>>();
                                //*DEBUG_MESSAGE.write().await = v.join(",");
                                app_state1.last_tick = Arc::new(new_last_tick);
                            }
                            if *app_state.read().await.market_ids != market_ids {
                                stream.close();
                                break;
                            }
                            match tx.unbounded_send(ticker.clone()) {
                                Ok(_) => (),
                                Err(err) => error!("{}", err),
                            }
                        }
                        None => {
                            stream.close();
                            break;
                        }
                    }
                }
            } else {
                tokio::task::yield_now().await;
            }
        }
    });
    (handle, rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test1() {
        let a = tokio::spawn(async {
            loop {
                println!("1");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
        a.abort();
        println!("{:?}", a.await);
    }

    #[tokio::test]
    async fn test_stream() {
        let upbit_service = Arc::new(UpbitServiceSimple::new("", ""));

        let app_state = Arc::new(RwLock::new(AppState {
            accounts: Arc::new(HashSet::from_iter(vec![Account {
                currency: "KRW-BTC".to_owned(),
                balance: Decimal::from(1),
                ..Account::default()
            }])),
            ..AppState::new()
        }));

        ticker_updater(app_state.clone(), upbit_service.clone()).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let mut market_ids = upbit_service
            .market_list()
            .await
            .unwrap()
            .into_iter()
            .map(|x| x.market)
            .filter(|x| x.starts_with("KRW"))
            .collect_vec();

        app_state.write().await.market_ids = Arc::new(market_ids.clone());
        println!("update");
        tokio::time::sleep(Duration::from_secs(2)).await;
        market_ids.remove(0);
        app_state.write().await.market_ids = Arc::new(market_ids);
        println!("update");
        //s.await;
    }
}
