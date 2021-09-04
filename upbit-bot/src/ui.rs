use std::{
    collections::HashMap,
    io::{self, Stdout},
    time::Duration,
};
use anyhow::Result;
use futures::{
    channel::mpsc::{self, Receiver},
    SinkExt,
};
use itertools::Itertools;
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
    debug_messages: &Vec<String>,
    req_remains: &HashMap<String, (u32, u32)>,
) -> Result<UiState> {
    let mut new_ui_state = ui_state.clone();
    let lines = l.lines();
    terminal.draw(|f| {
        let main_layout = layout::Layout::default()
            .direction(layout::Direction::Horizontal)
            .constraints(
                [
                    layout::Constraint::Percentage(70),
                    layout::Constraint::Percentage(30),
                ]
                .as_ref(),
            )
            .split(f.size());

        let console_layout = layout::Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints(
                [
                    layout::Constraint::Percentage(60),
                    layout::Constraint::Percentage(30),
                ]
                .as_ref(),
            )
            .split(main_layout[0]);

        new_ui_state.height = console_layout[0].height as i32;
        let border_size = 2;
        new_ui_state.height -= border_size;

        let console_block = widgets::Block::default()
            .title("──< UpBit Console >──")
            .borders(widgets::Borders::ALL);
        let console_paragraph = widgets::Paragraph::new(lines.join("\n"))
            .block(console_block)
            .alignment(layout::Alignment::Left)
            .scroll((ui_state.scroll as u16, 0));
        f.render_widget(console_paragraph, console_layout[0]);

        let debug_block = widgets::Block::default()
            .title("──< Debug >──")
            .borders(widgets::Borders::ALL);
        let debug_paragraph = widgets::Paragraph::new(debug_messages.join("\n"))
            .block(debug_block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true });
        f.render_widget(debug_paragraph, console_layout[1]);

        let mut req_remains = req_remains
            .clone()
            .iter()
            .map(|(group, (min, sec))| format!("{} -> min: {}, sec: {}", group, min, sec))
            .collect_vec();
        req_remains.sort();

        let req_block = widgets::Block::default()
            .title("──< Req-Remain >──")
            .borders(widgets::Borders::ALL);
        let req_paragraph = widgets::Paragraph::new(req_remains.join("\n"))
            .block(req_block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true });
        f.render_widget(req_paragraph, main_layout[1]);
    })?;
    Ok(new_ui_state)
}
