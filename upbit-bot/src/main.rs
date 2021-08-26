//#![deny(warnings)]
mod domain;
mod ui;
mod upbit;

use anyhow::Result;
use async_lock::RwLock;
use async_trait::async_trait;
use format_num::format_num;

use crossterm::event::{self, KeyModifiers};
//use chrono::Local;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};

//use static_init::dynamic;
use futures::StreamExt;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;


use std::collections::HashSet;

use std::iter::FromIterator;
use std::num::NonZeroU32;



use std::sync::{Arc};
use std::time::Duration;
use tokio::spawn;
use ui::ToLines;

use upbit::*;

use crate::domain::{get_all_tickers, AppState};


struct UpbitRateLimiterService<U: UpbitService> {
    order_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    exchange_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    quotation_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    upbit_service: Arc<U>,
}

impl<U: UpbitService> UpbitRateLimiterService<U> {
    fn new(
        upbit_service: Arc<U>,
        order_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
        exchange_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
        quotation_rate_limiters: Arc<Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>>,
    ) -> UpbitRateLimiterService<U> {
        UpbitRateLimiterService {
            order_rate_limiters,
            exchange_rate_limiters,
            quotation_rate_limiters,
            upbit_service,
        }
    }
}

#[async_trait]
impl<U: UpbitService> UpbitService for UpbitRateLimiterService<U> {
    async fn market_list(&self) -> Result<Vec<Market>, Error> {
        for rate_limiter in self.quotation_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service.market_list().await
    }

    async fn market_ticker_list(&self, market_ids: Vec<String>) -> Result<Vec<TradeTick>, Error> {
        for rate_limiter in self.quotation_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service.market_ticker_list(market_ids).await
    }

    async fn candles_minutes(
        &self,
        unit: MinuteUnit,
        market_id: &str,
        count: u8,
    ) -> Result<Vec<Candle>, Error> {
        for rate_limiter in self.quotation_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service
            .candles_minutes(unit, market_id, count)
            .await
    }

    async fn accounts(&self, access_key: &str, secret_key: &str) -> Result<Vec<Account>, Error> {
        for rate_limiter in self.exchange_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service.accounts(access_key, secret_key).await
    }

    async fn orders_chance(
        &self,
        access_key: &str,
        secret_key: &str,
        market_id: &str,
    ) -> Result<OrderChance, Error> {
        for rate_limiter in self.exchange_rate_limiters.iter() {
            rate_limiter.until_ready().await;
        }
        self.upbit_service
            .orders_chance(access_key, secret_key, market_id)
            .await
    }
}

#[allow(dead_code)]
fn create_limiter(
    per_second: u32,
    per_minute: u32,
) -> Vec<RateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
    vec![
        RateLimiter::direct(Quota::per_second(NonZeroU32::new(per_second).unwrap())),
        RateLimiter::direct(Quota::per_minute(NonZeroU32::new(per_minute).unwrap())),
    ]
}

impl ToLines for AppState {
    fn lines(&self) -> Vec<String> {
        let mut result = self
            .accounts
            .iter()
            .map(|a| {
                let v = self
                    .last_tick
                    .iter()
                    .find(|t| t.market == format!("{}-{}", a.unit_currency, a.currency));
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
            .collect::<Vec<_>>();
        result.sort();
        result
    }
}

impl ToLines for Vec<String> {
    fn lines(&self) -> Vec<String> {
        self.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let order_rate_limiters = create_limiter(8, 200);
    let exchange_rate_limiters = create_limiter(30, 900);
    let quotation_rate_limiters = create_limiter(10, 600);
    //let upbit_service = Arc::new(UpbitRateLimiterService::new(
    //    Arc::new(UpbitServiceDummyAccount::new()),
    //    Arc::new(order_rate_limiters),
    //    Arc::new(exchange_rate_limiters),
    //    Arc::new(quotation_rate_limiters),
    //));
    let upbit_service = Arc::new(UpbitRateLimiterService::new(
        Arc::new(UpbitServiceSimple::new()),
        Arc::new(order_rate_limiters),
        Arc::new(exchange_rate_limiters),
        Arc::new(quotation_rate_limiters),
    ));
    let access_key = "nJYLpyEglbwNGd2DHIjJ1rBCuchEtnL2PXjIdKRO";
    let secret_key = "E7Fg5LexgdfmXwLYtxk7P7r3L4FzsfkZkdNhTyw5";
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

    let tick_rate = Duration::from_millis(250);
    let mut rx = ui::start_ui_ticker(tick_rate);
    let mut ui_state = ui::UiState::new();

    // last tick updater
    {
        let app_state1 = app_state.clone();
        let upbit_service1 = upbit_service.clone();
        spawn(async move {
            loop {
                let tickers = get_all_tickers(upbit_service1.clone()).await;
                match tickers {
                    Ok(xs) => app_state1.write().await.last_tick = Arc::new(xs),
                    Err(e) => {
                        println!("{}", e);
                        break;
                    }
                }
            }
        });
    }

    // accounts updater
    {
        let app_state1 = app_state.clone();
        let upbit_service1 = upbit_service.clone();
        spawn(async move {
            loop {
                let accounts = upbit_service1.accounts(access_key, secret_key).await;
                match accounts {
                    Ok(xs) => {
                        app_state1.clone().write().await.accounts = Arc::new(HashSet::from_iter(xs))
                    }
                    Err(e) => {
                        println!("{}", e);
                        break;
                    }
                }
            }
        });
    }

    // buyer
    {}

    loop {
        //ui_state = ui::draw(&mut terminal, ui_state, &app_state, "")
        match rx.next().await {
            Some(crate::ui::Event::Tick) => {
                ui_state = ui::draw(&mut terminal, ui_state, &(*app_state.read().await), "")
                    .await
                    .unwrap();
            }
            Some(crate::ui::Event::UiEvent(e)) => match e {
                event::Event::Key(key_event) => match key_event.code {
                    event::KeyCode::Char('q') => {
                        ui::rollback_ui(&mut terminal)?;
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
