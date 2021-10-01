use anyhow::Result;
use crossterm::event::{self, KeyModifiers};
use futures::{
    channel::mpsc::{self, Receiver},
    SinkExt,
};
use itertools::Itertools;
use std::sync::Arc;
use std::{
    collections::HashMap,
    io::{self, Stdout},
    time::Duration,
};
use tracing::error;
use tui::{
    backend::{self, CrosstermBackend},
    layout, widgets, Terminal,
};

#[derive(Clone, Debug)]
pub struct UiState {
    pub is_shutdown: bool,
    pub input_mode: InputMode,
    pub message_vscroll: i32,
    pub message_hscroll: i32,
    pub message_height: i32,
    pub message_width: i32,
    pub account_info: Arc<Vec<String>>,
    pub state_info: Arc<Vec<String>>,
    pub candle_info: Arc<Vec<String>>,
    pub req_remain_info: Arc<HashMap<String, (u32, u32)>>,
    pub message_info: Arc<Vec<String>>,
}

impl UiState {
    pub fn new() -> UiState {
        UiState {
            is_shutdown: false,
            input_mode: InputMode::Normal,
            message_vscroll: 0,
            message_hscroll: 0,
            message_height: 0,
            message_width: 0,
            account_info: Arc::new(vec![]),
            state_info: Arc::new(vec![]),
            candle_info: Arc::new(vec![]),
            req_remain_info: Arc::new(HashMap::new()),
            message_info: Arc::new(vec![]),
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputMode {
    Normal,
    Command,
}

#[derive(Clone, Debug)]
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
                        error!("{} ({}:{})", file!(), line!(), e);
                    }
                }
            }
            if last_tick.elapsed() >= tick_rate {
                match tx.send(Event::Tick).await {
                    Ok(_) => (),
                    Err(e) => {
                        error!("{} ({}:{})", e, file!(), line!());
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

pub async fn draw(terminal: &mut MyTerminal, ui_state: UiState) -> Result<UiState> {
    let mut new_ui_state = ui_state.clone();
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

        let left_layout = layout::Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints(
                [
                    layout::Constraint::Percentage(50),
                    layout::Constraint::Percentage(50),
                ]
                .as_ref(),
            )
            .split(main_layout[0]);

        let right_layout = layout::Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints(
                [
                    layout::Constraint::Percentage(40),
                    layout::Constraint::Percentage(30),
                    layout::Constraint::Percentage(30),
                ]
                .as_ref(),
            )
            .split(main_layout[1]);

        let border_size = 2;
        new_ui_state.message_height = left_layout[1].height as i32 - border_size;
        new_ui_state.message_width = left_layout[1].width as i32 - border_size;

        let account_block = widgets::Block::default()
            .title("──< Account >──")
            .borders(widgets::Borders::ALL);
        let account_paragraph = widgets::Paragraph::new(ui_state.account_info.join("\n"))
            .block(account_block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true });
        f.render_widget(account_paragraph, left_layout[0]);

        let state_block = widgets::Block::default()
            .title("──< State (Count) >──")
            .borders(widgets::Borders::ALL);
        let state_paragraph = widgets::Paragraph::new(ui_state.state_info.join("\n"))
            .block(state_block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true });
        f.render_widget(state_paragraph, right_layout[0]);

        let candle_block = widgets::Block::default()
            .title("──< Candle >──")
            .borders(widgets::Borders::ALL);
        let candle_paragraph = widgets::Paragraph::new(ui_state.candle_info.join("\n"))
            .block(candle_block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true });
        f.render_widget(candle_paragraph, right_layout[1]);

        let message_block = widgets::Block::default()
            .title("──< Message >──")
            .borders(widgets::Borders::ALL);
        let message_paragraph = widgets::Paragraph::new(ui_state.message_info.join("\n"))
            .block(message_block)
            .alignment(layout::Alignment::Left)
            .scroll((
                ui_state.message_vscroll as u16,
                ui_state.message_hscroll as u16,
            ));
        f.render_widget(message_paragraph, left_layout[1]);

        let req_remain_block = widgets::Block::default()
            .title("──< Req-Remain >──")
            .borders(widgets::Borders::ALL);
        let req_remain_text = ui_state
            .req_remain_info
            .clone()
            .iter()
            .map(|(group, (min, sec))| format!("{} -> min: {}, sec: {}", group, min, sec))
            .collect_vec()
            .join("\n");
        let req_remain_paragraph = widgets::Paragraph::new(req_remain_text)
            .block(req_remain_block)
            .alignment(layout::Alignment::Left)
            .wrap(widgets::Wrap { trim: true });
        f.render_widget(req_remain_paragraph, right_layout[2]);
    })?;
    Ok(new_ui_state)
}

pub async fn handle_input(
    ui_state: UiState,
    event: Event,
    terminal: &mut MyTerminal,
) -> Result<UiState> {
    let mut ui_state = ui_state;
    match event {
        crate::ui::Event::Tick => {
            ui_state = draw(terminal, ui_state).await.unwrap();
            Ok(ui_state)
        }
        crate::ui::Event::UiEvent(e) => match e {
            event::Event::Key(key_event) => match key_event.code {
                event::KeyCode::Char('q') => {
                    rollback_ui(terminal)?;
                    ui_state.is_shutdown = true;
                    Ok(ui_state)
                }
                event::KeyCode::Char('c')
                    if (key_event.modifiers.contains(KeyModifiers::CONTROL)) =>
                {
                    rollback_ui(terminal)?;
                    ui_state.is_shutdown = true;
                    Ok(ui_state)
                }
                event::KeyCode::Char('h') => {
                    ui_state.message_hscroll -= 2;
                    ui_state.message_hscroll = ui_state.message_hscroll.max(0);
                    Ok(ui_state)
                }
                event::KeyCode::Char('l') => {
                    ui_state.message_hscroll += 2;
                    let message_info_width = ui_state
                        .message_info
                        .iter()
                        .map(|x| x.len())
                        .max()
                        .unwrap_or(0);
                    ui_state.message_hscroll = ui_state
                        .message_hscroll
                        .min(message_info_width as i32 - ui_state.message_width)
                        .max(0);
                    Ok(ui_state)
                }
                event::KeyCode::Char('k') => {
                    ui_state.message_vscroll -= 1;
                    ui_state.message_vscroll = ui_state.message_vscroll.max(0);
                    Ok(ui_state)
                }
                event::KeyCode::Char('j') => {
                    ui_state.message_vscroll += 1;
                    ui_state.message_vscroll = ui_state
                        .message_vscroll
                        .min(ui_state.message_info.len() as i32 - ui_state.message_height)
                        .max(0);
                    Ok(ui_state)
                }
                _ => Ok(ui_state),
            },
            _ => Ok(ui_state),
        },
    }
}
