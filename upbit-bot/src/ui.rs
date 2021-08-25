use std::{io, time::Duration};

use anyhow::Result;
use crossterm::event::{self, KeyModifiers};
use futures::{
    channel::mpsc::{self, Sender},
    SinkExt,
};
use futures::StreamExt;
use tui::{
    backend::{self, CrosstermBackend},
    layout, widgets, Terminal,
};

pub enum Event {
    Tick,
    UiEvent(crossterm::event::Event),
}

fn start_ticker(tick_rate: Duration, mut tx: Sender<Event>) {
    tokio::spawn(async move {
        let mut last_tick = tokio::time::Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout).unwrap() {
                if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
                    tx.send(Event::UiEvent(crossterm::event::Event::Key(key)))
                        .await
                        .unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                match tx.send(Event::Tick).await {
                    Ok(_) => (),
                    Err(_) => break,
                }
                last_tick = tokio::time::Instant::now();
            }
        }
    });
}

pub async fn run(_state: async_lock::RwLock<Vec<String>>) -> Result<()> {
    let mut stdout = io::stdout();
    let (tx, mut rx) = mpsc::channel(0);
    let tick_rate = Duration::from_millis(250);

    crossterm::terminal::enable_raw_mode()?;
    //crossterm::execute!(
    //    stdout,
    //    crossterm::terminal::EnterAlternateScreen,
    //    event::EnableMouseCapture
    //)?;

    start_ticker(tick_rate, tx);

    let backend = backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let mut scroll = 0_i32;
    let mut v_size = 0_i32;

    fn rollback_console(t: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            t.backend_mut(),
            crossterm::terminal::LeaveAlternateScreen,
            event::DisableMouseCapture
        )?;
        t.show_cursor()?;
        Ok(())
    }

    loop {
        let now = chrono::Local::now().to_rfc3339();
        let lines = (0..50)
            .map(|n| format!("[{}] {}", now, n))
            .collect::<Vec<_>>();
        //let market_tickers = match upbit_service.market_list().await {
        //    Ok(markets) => {
        //        let filtered_markets = markets
        //            .into_iter()
        //            .filter(|x| x.market.starts_with("KRW"))
        //            .map(|x| x.market)
        //            .collect::<Vec<_>>();
        //        match upbit_service.market_ticker_list(filtered_markets).await {
        //            Ok(market_tickers) => market_tickers,
        //            Err(e) => {
        //                println!("{:?}", e);
        //                vec![]
        //            }
        //        }
        //    }
        //    Err(e) => {
        //        println!("{:?}", e);
        //        vec![]
        //    }
        //};
        //let accounts_result = upbit_service.accounts(access_key, secret_key).await;
        //let lines = {
        //    match accounts_result {
        //        Ok(accounts) => accounts
        //            .iter()
        //            .map(|x| format!("{}", x))
        //            .collect::<Vec<_>>(),
        //        Err(_) => vec!["".to_owned()],
        //    }
        //};
        let len_lines = lines.len() as i32;
        let debug_text = "";
        terminal.draw(|f| {
            v_size = f.size().height as i32;
            let block = widgets::Block::default()
                .title(format!("──< UpBit Console > -- [{}]", debug_text))
                .borders(widgets::Borders::ALL);
            let border_size = 2;
            v_size -= border_size;
            let paragraph = widgets::Paragraph::new(lines.join("\n"))
                .block(block)
                .alignment(layout::Alignment::Left)
                .wrap(widgets::Wrap { trim: true })
                .scroll((scroll as u16, 0));
            f.render_widget(paragraph, f.size());
        })?;
        match rx.next().await {
            Some(crate::ui::Event::Tick) => (),
            Some(crate::ui::Event::UiEvent(e)) => match e {
                event::Event::Key(key_event) => match key_event.code {
                    event::KeyCode::Char('q') => {
                        rollback_console(&mut terminal)?;
                        break;
                    }
                    event::KeyCode::Char('c')
                        if (key_event.modifiers.contains(KeyModifiers::CONTROL)) =>
                    {
                        rollback_console(&mut terminal)?;
                        break;
                    }
                    event::KeyCode::Char('k') => {
                        scroll -= 1;
                        scroll = scroll.max(0);
                    }
                    event::KeyCode::Char('j') => {
                        scroll += 1;
                        scroll = scroll.min(len_lines - v_size).max(0);
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
