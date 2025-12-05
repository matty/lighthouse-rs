use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;

use lighthouse_core::bluetooth::{scan_process_and_save_with_json, POWERON_COMMAND, STANDBY_COMMAND};
use lighthouse_core::config::load_devices;
use lighthouse_core::models::DeviceInfo;

pub async fn run_tui() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear once on startup to avoid any leftover console content causing garbled first render
    terminal.clear()?;

    let res = run_app(&mut terminal).await;

    // Restore terminal
    disable_raw_mode().ok();
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).ok();

    res
}

struct AppState {
    devices: Vec<DeviceInfo>,
    selected: usize,
    status: String,
    last_refresh: Instant,
}

impl AppState {
    fn new() -> Self {
        Self {
            devices: Vec::new(),
            selected: 0,
            status: "Press 'r' to scan for devices".to_string(),
            last_refresh: Instant::now(),
        }
    }
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn Error>> {
    let mut app = AppState::new();

    // Initial load from cache if available
    match load_devices() {
        Ok(devs) => {
            app.devices = devs;
            if app.devices.is_empty() {
                app.status = "No cached devices. Press 'r' to scan.".into();
            } else {
                app.status = format!("Loaded {} cached devices", app.devices.len());
            }
        }
        Err(_) => {
            app.status = "No cache found. Press 'r' to scan.".into();
        }
    }

    loop {
        // Draw UI
        terminal.draw(|f| {
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // header
                    Constraint::Min(1),     // list
                    Constraint::Length(3), // footer
                ])
                .split(size);

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled("lighthouse-rs", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" — TUI"),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Header"));
            f.render_widget(header, chunks[0]);

            // Device list
            let items: Vec<ListItem> = if app.devices.is_empty() {
                vec![ListItem::new("No devices. Press 'r' to scan.")]
            } else {
                app
                    .devices
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let marker = if i == app.selected { "> " } else { "  " };
                        let line = format!("{}{} — {}", marker, d.name, d.address);
                        ListItem::new(line)
                    })
                    .collect()
            };
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Devices"));
            f.render_widget(list, chunks[1]);

            // Footer/help (draw bordered block and split into two columns inside)
            let footer_area = chunks[2];
            let footer_block = Block::default().borders(Borders::ALL).title("Help");
            let inner = footer_block.inner(footer_area);
            f.render_widget(footer_block, footer_area);

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(inner);

            // Left: key bindings (short labels to avoid wrap)
            let keys_line = Line::from(vec![
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" quit  "),
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::raw(" rescan  "),
                Span::styled("p", Style::default().fg(Color::Yellow)),
                Span::raw(" power on  "),
                Span::styled("s", Style::default().fg(Color::Yellow)),
                Span::raw(" standby"),
            ]);
            let keys_para = Paragraph::new(keys_line).wrap(Wrap { trim: true });
            f.render_widget(keys_para, cols[0]);

            // Right: status (right-aligned)
            let status_para = Paragraph::new(Line::from(vec![
                Span::styled("Status:", Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {}", app.status)),
            ]))
            .alignment(Alignment::Right)
            .wrap(Wrap { trim: true });
            f.render_widget(status_para, cols[1]);
        })?;

        // Input handling with small tick
        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Up => {
                        if !app.devices.is_empty() {
                            if app.selected == 0 {
                                app.selected = app.devices.len() - 1;
                            } else {
                                app.selected -= 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !app.devices.is_empty() {
                            app.selected = (app.selected + 1) % app.devices.len();
                        }
                    }
                    KeyCode::Char('r') => {
                        app.status = "Scanning for devices...".into();
                        // Refresh UI once before starting async op
                        terminal.draw(|_| {}).ok();
                        if let Err(e) = scan_process_and_save_with_json(0xFF, false).await {
                            app.status = format!("Scan failed: {}", e);
                        } else {
                            match load_devices() {
                                Ok(devs) => {
                                    app.selected = 0;
                                    app.devices = devs;
                                    app.status = format!("Found {} devices", app.devices.len());
                                }
                                Err(e) => app.status = format!("Failed to load cache: {}", e),
                            }
                        }
                        app.last_refresh = Instant::now();
                    }
                    KeyCode::Char('p') => {
                        app.status = "Powering on all devices...".into();
                        terminal.draw(|_| {}).ok();
                        match scan_process_and_save_with_json(POWERON_COMMAND, false).await {
                            Ok(_) => app.status = "Power on command sent".into(),
                            Err(e) => app.status = format!("Power on failed: {}", e),
                        }
                    }
                    KeyCode::Char('s') => {
                        app.status = "Putting all devices to standby...".into();
                        terminal.draw(|_| {}).ok();
                        match scan_process_and_save_with_json(STANDBY_COMMAND, false).await {
                            Ok(_) => app.status = "Standby command sent".into(),
                            Err(e) => app.status = format!("Standby failed: {}", e),
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
