use std::time::Duration;

use futures::{channel::mpsc::Sender, SinkExt};

pub enum Event {
    Tick,
    UiEvent(crossterm::event::Event),
}

pub fn start_ticker(tick_rate: Duration, mut tx: Sender<Event>) {
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
