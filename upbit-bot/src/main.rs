//#![deny(warnings)]
mod domain;
mod ui;
mod upbit;

use crate::domain::*;
use anyhow::Result;
use async_lock::RwLock;
use chrono::prelude::*;
use chrono::DurationRound;
use crossterm::event::{self, KeyModifiers};
use dotenv::dotenv;
use futures::StreamExt;
use itertools::Itertools;
use rust_decimal::Decimal;
use static_init::dynamic;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::FromIterator;

use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use ui::ToLines;
use upbit::*;

#[dynamic]
static DEBUG_MESSAGES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));
#[dynamic]
static REQ_REMAINS: Arc<RwLock<HashMap<String, (u32, u32)>>> =
    Arc::new(RwLock::new(HashMap::new()));

pub async fn append_debug_message(message: &str) {
    let mut messages = DEBUG_MESSAGES.write().await;
    let now = chrono::Local::now();
    messages.insert(0, format!("[{}] {}", now.to_rfc3339(), message));
    if messages.len() > 5 {
        let len = messages.len();
        messages.remove(len - 1);
    }
}

pub async fn insert_req_remain(group: &str, min_remain: u32, sec_remain: u32) {
    let mut req_remain = REQ_REMAINS.write().await;
    req_remain.insert(group.to_owned(), (min_remain, sec_remain));
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let upbit_service = Arc::new(UpbitServiceSimple::new());
    let buyer_service = Arc::new(BuyerServiceSimple::new(upbit_service.clone()));

    let access_key = std::env::var("ACCESS_KEY")
        .map_err(|_| anyhow::format_err!("ACCESS_KEY was not found in environment var"))?;
    let secret_key = std::env::var("SECRET_KEY")
        .map_err(|_| anyhow::format_err!("SECRET_KEY was not found in environment var"))?;

    let app_state = Arc::new(RwLock::new(AppState {
        accounts: Arc::new(HashSet::from_iter(vec![Account {
            currency: "KRW-BTC".to_owned(),
            balance: Decimal::from(1),
            ..Account::default()
        }])),
        ..AppState::new()
    }));
    let mut terminal = ui::create_terminal()?;
    ui::start_ui(&mut terminal)?;

    let tick_rate = Duration::from_millis(25);
    let mut rx = ui::start_ui_ticker(tick_rate);
    let mut ui_state = ui::UiState::new();

    // candle updater

    let _candle_updater = candle_updater(app_state.clone(), upbit_service.clone()).await;

    let _market_list_updater = market_list_updater(app_state.clone(), upbit_service.clone()).await;

    let _account_updater = account_updater(
        app_state.clone(),
        upbit_service.clone(),
        access_key,
        secret_key,
    )
    .await;

    let _ticker_updater = ticker_updater(
        app_state.clone(),
        upbit_service.clone(),
        buyer_service.clone(),
    )
    .await;

    // cli start
    loop {
        match rx.next().await {
            Some(crate::ui::Event::Tick) => {
                ui_state = ui::draw(
                    &mut terminal,
                    ui_state,
                    &(*app_state.read().await),
                    &*DEBUG_MESSAGES.clone().read().await,
                    &*REQ_REMAINS.clone().read().await,
                )
                .await
                .unwrap();
            }
            Some(crate::ui::Event::UiEvent(e)) => match e {
                event::Event::Key(key_event) => match key_event.code {
                    event::KeyCode::Char('q') => {
                        ui::rollback_ui(&mut terminal)?;
                        REQ_REMAINS
                            .read()
                            .await
                            .iter()
                            .for_each(|x| println!("{}, {}, {}", x.0, x.1 .0, x.1 .1));
                        break;
                    }
                    event::KeyCode::Char('c')
                        if (key_event.modifiers.contains(KeyModifiers::CONTROL)) =>
                    {
                        ui::rollback_ui(&mut terminal)?;
                        break;
                    }
                    event::KeyCode::Char('k') => {
                        ui_state.scroll -= 1;
                        ui_state.scroll = ui_state.scroll.max(0);
                    }
                    event::KeyCode::Char('l')
                        if (key_event.modifiers.contains(KeyModifiers::CONTROL)) =>
                    {
                        terminal.clear()?;
                        *DEBUG_MESSAGES.write().await = vec![];
                        *REQ_REMAINS.write().await = HashMap::new();
                    }
                    event::KeyCode::Char('j') => {
                        ui_state.scroll += 1;
                        ui_state.scroll = ui_state
                            .scroll
                            .min(app_state.read().await.lines().len() as i32 - ui_state.height)
                            .max(0);
                    }
                    _ => (),
                },
                _ => (),
            },
            None => break,
        }
    }
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
                    append_debug_message(&format!("{}", e)).await;
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
                        let mut new_history = (&*app_state.write().await.history).clone();
                        new_history.insert(market_id.to_owned(), Arc::new(xs));
                        app_state.write().await.history = Arc::new(new_history);
                    }
                    Err(e) => {
                        append_debug_message(&format!("{}", e)).await;
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
    access_key: String,
    secret_key: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let accounts = upbit_service
                .accounts(access_key.as_str(), secret_key.as_str())
                .await;
            match accounts {
                Ok(xs) => {
                    app_state.clone().write().await.accounts = Arc::new(HashSet::from_iter(xs))
                }
                Err(e) => {
                    append_debug_message(&format!("{}", e)).await;
                }
            }
        }
    })
}

async fn ticker_updater<U: UpbitService + 'static, B: BuyerService + 'static>(
    app_state: Arc<RwLock<AppState>>,
    upbit_service: Arc<U>,
    buyer_service: Arc<B>,
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

            if market_ids.len() > 0 {
                let mut stream = upbit_service
                    .clone()
                    .ticker_stream(&market_ids)
                    .await
                    .unwrap();

                loop {
                    let ticker = stream.next().await.unwrap();
                    {
                        let mut app_state1 = app_state.write().await;
                        let mut new_last_tick = app_state1.last_tick.as_ref().clone();
                        new_last_tick.insert(ticker.code.clone(), ticker);
                        //let v = new_last_tick.keys().map(|x| x.clone()).collect::<Vec<_>>();
                        //*DEBUG_MESSAGE.write().await = v.join(",");
                        app_state1.last_tick = Arc::new(new_last_tick);
                    }
                    buyer_service.process(&*app_state.read().await).await;
                    if *app_state.read().await.market_ids != market_ids {
                        stream.close();
                        break;
                    }
                }
            } else {
                tokio::task::yield_now().await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_stream() {
        let upbit_service = Arc::new(UpbitServiceSimple::new());
        let buyer_service = Arc::new(BuyerServiceSimple::new(upbit_service.clone()));

        let app_state = Arc::new(RwLock::new(AppState {
            accounts: Arc::new(HashSet::from_iter(vec![Account {
                currency: "KRW-BTC".to_owned(),
                balance: Decimal::from(1),
                ..Account::default()
            }])),
            ..AppState::new()
        }));

        let _s = ticker_updater(app_state.clone(), upbit_service.clone(), buyer_service.clone()).await;
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
