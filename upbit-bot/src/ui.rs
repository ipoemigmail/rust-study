use std::{
    borrow::BorrowMut,
    cell::{RefCell, RefMut},
    io::{self, Stdout},
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;


use futures::{
    channel::mpsc::{self, Receiver},
    SinkExt,
};
use tui::{
    backend::{self, CrosstermBackend},
    layout, widgets, Terminal,
};

#[derive(Clone, Debug)]
pub struct UiState {
    pub scroll: i32,
    pub height: i32,
}

impl UiState {
    pub fn new() -> UiState {
        UiState {
            scroll: 0,
            height: 0,
        }
    }
}

pub trait ToLines {
    fn lines(&self) -> Vec<String>;
}

pub enum Event {
    Tick,
    UiEvent(crossterm::event::Event),
}

pub fn start_ui_ticker(tick_rate: Duration) -> Receiver<Event> {
    let (mut tx, rx) = mpsc::channel(0);
    tokio::spawn(async move {
        let mut last_tick = tokio::time::Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if crossterm::event::poll(timeout).unwrap() {
                match crossterm::event::read() {
                    Ok(crossterm::event::Event::Key(key)) => {
                        if let Err(_e) = tx
                            .send(Event::UiEvent(crossterm::event::Event::Key(key)))
                            .await
                        {
                            break;
                        }
                    }
                    Ok(_) => (),
                    Err(e) => {
                        println!("{}:{} - {}", file!(), line!(), e);
                        break;
                    }
                }
            }
            if last_tick.elapsed() >= tick_rate {
                match tx.send(Event::Tick).await {
                    Ok(_) => (),
                    Err(_e) => {
                        break;
                    }
                }
                last_tick = tokio::time::Instant::now();
            }
        }
    });
    rx
}

type MyTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn create_terminal() -> Result<MyTerminal> {
    let stdout = io::stdout();
    let backend = backend::CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

pub fn start_ui(terminal: &mut MyTerminal) -> Result<()> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        //event::EnableMouseCapture
    )?;
    terminal.clear()?;
    Ok(())
}

pub fn rollback_ui(terminal: &mut MyTerminal) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        //event::DisableMouseCapture
    )?;
    terminal.clear()?;
    terminal.show_cursor()?;
    Ok(())
}

pub async fn draw<L: ToLines>(
    terminal: &mut MyTerminal,
    ui_state: UiState,
    l: &L,
    debug_text: &str,
) -> Result<UiState> {
    let mut new_ui_state = ui_state.clone();
    let lines = l.lines();
    terminal.draw(|f| {
        new_ui_state.height = f.size().height as i32;
        let block = widgets::Block::default()
            .title(format!("──< UpBit Console > -- [{}]", debug_text))
            .borders(widgets::Borders::ALL);
        let border_size = 2;
        new_ui_state.height -= border_size;
        let paragraph = widgets::Paragraph::new(lines.join("\n"))
            .block(block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true })
            .scroll((ui_state.scroll as u16, 0));
        f.render_widget(paragraph, f.size());
    })?;
    Ok(new_ui_state)
}
