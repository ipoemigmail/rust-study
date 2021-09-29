//#![deny(warnings)]
mod app;
mod buy;
mod sell;
mod simulation;
mod ui;
mod upbit;
mod util;

use crate::app::*;

use anyhow::Result;
use app::ToLines;
use buy::*;
use chrono::prelude::*;
use chrono::DurationRound;
use dotenv::dotenv;
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedReceiver;
use futures::StreamExt;
use itertools::*;
use sell::*;
use simulation::*;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::error;
use upbit::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let access_key = std::env::var("ACCESS_KEY")
        .map_err(|_| anyhow::format_err!("ACCESS_KEY was not found in environment var"))?;
    let secret_key = std::env::var("SECRET_KEY")
        .map_err(|_| anyhow::format_err!("SECRET_KEY was not found in environment var"))?;

    let app_state_service = Arc::new(AppStateServiceSimple::new());
    let upbit_service = Arc::new(UpbitServiceSimple::new(&access_key, &secret_key));
    let upbit_service = Arc::new(
        UpbitServiceDummyAccount::new_with_amount(
            1_000_000,
            upbit_service,
            app_state_service.clone(),
        )
        .await,
    );
    let buyer_service = Arc::new(BuyerServiceSimple::new(upbit_service.clone()));
    let seller_service = Arc::new(SellerServiceSimple::new(upbit_service.clone()));

    tracing_subscriber::fmt()
        .with_writer(AppStateWriter::new(app_state_service.clone()))
        .with_ansi(false)
        .init();
    //let a = upbit_service.accounts(&access_key, &secret_key).await.unwrap();
    //info!("{}", serde_json::to_string(&a).unwrap());

    let mut terminal = ui::create_terminal()?;
    ui::start_ui(&mut terminal)?;

    let tick_rate = Duration::from_millis(25);
    let mut rx = ui::start_ui_ticker(tick_rate);
    let mut ui_state = ui::UiState::new();

    let candle_updater = candle_updater(app_state_service.clone(), upbit_service.clone()).await;

    let market_list_updater =
        market_list_updater(app_state_service.clone(), upbit_service.clone()).await;

    let account_updater = account_updater(app_state_service.clone(), upbit_service.clone()).await;

    let (ticker_updater, mut ticker_stream) =
        ticker_updater(app_state_service.clone(), upbit_service.clone()).await;

    let _app_state_service = app_state_service.clone();
    let buyer_handle = tokio::spawn(async move {
        loop {
            match ticker_stream.next().await {
                Some(_) => {
                    let app_state = _app_state_service.clone().state().await;
                    seller_service.process(&app_state).await;
                    buyer_service.process(&app_state).await;
                }
                None => (),
            }
        }
    });

    // cli start
    let _app_state_service = app_state_service.clone();
    loop {
        ui_state.main_messages = Arc::new(_app_state_service.state().await.lines());
        ui_state.req_remains = upbit_service.remain_req().await;
        ui_state.debug_messages = _app_state_service.log_messages().await.clone();
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
                app_state_service.set_shutdown(ui_state.is_shutdown).await;
                if ui_state.is_shutdown {
                    break;
                }
            }
            None => break,
        }
    }
    let is_shutdown = app_state_service.is_shutdown().await;
    candle_updater.abort();
    market_list_updater.abort();
    account_updater.abort();
    ticker_updater.abort();
    buyer_handle.abort();
    println!("is_shutdown: {}", is_shutdown);
    Ok(())
}

async fn market_list_updater<S: AppStateService + 'static, U: UpbitService + 'static>(
    app_state_service: Arc<S>,
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
                    app_state_service.set_market_ids(market_ids).await
                }
                Err(e) => {
                    error!("{}", e)
                }
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}

async fn candle_updater<S: AppStateService + 'static, U: UpbitService + 'static>(
    app_state_service: Arc<S>,
    upbit_service: Arc<U>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let market_ids = app_state_service
                .market_ids()
                .await
                .iter()
                .map(|x| x.clone())
                .collect_vec();

            for market_id in market_ids.clone() {
                match upbit_service
                    .candles_minutes(MinuteUnit::_1, &market_id, 200)
                    .await
                {
                    Ok(xs) => {
                        app_state_service
                            .update_candles(move |v: Arc<HashMap<String, Arc<Vec<Candle>>>>| {
                                let mut new_candles = v.as_ref().clone();
                                new_candles.insert(market_id.to_owned(), Arc::new(xs));
                                new_candles
                            })
                            .await
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

async fn account_updater<S: AppStateService + 'static, U: UpbitService + 'static>(
    app_state_service: Arc<S>,
    upbit_service: Arc<U>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let accounts = upbit_service.accounts().await;
            match accounts {
                Ok(xs) => {
                    app_state_service
                        .set_accounts(HashMap::from_iter(
                            xs.into_iter().map(|x| (x.currency.clone(), x)),
                        ))
                        .await
                }
                Err(e) => {
                    error!("{}", e)
                }
            }
        }
    })
}

async fn ticker_updater<S: AppStateService + 'static, U: UpbitService + 'static>(
    app_state_service: Arc<S>,
    upbit_service: Arc<U>,
) -> (JoinHandle<()>, UnboundedReceiver<TickerWs>) {
    let (tx, rx) = mpsc::unbounded();
    let handle = tokio::spawn(async move {
        loop {
            let market_ids = app_state_service
                .market_ids()
                .await
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
                            app_state_service
                                .update_last_tick(|v: Arc<HashMap<String, TickerWs>>| {
                                    let mut new_last_tick = v.as_ref().clone();
                                    new_last_tick.insert(ticker.code.clone(), ticker.clone());
                                    new_last_tick
                                })
                                .await;

                            if *app_state_service.market_ids().await != market_ids {
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
}
