use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    Terminal,
    style::{Style, Color},
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::{Duration, Instant};
use sysinfo::System;
use crate::check_port;

pub fn run_dashboard() -> Result<(), Box<dyn std::error::Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut sys = System::new_all();
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();
    
    // Subscribe to Event Bus
    let mut event_rx = sunday_core::GLOBAL_BUS.subscribe();
    let mut last_event_msg = String::from("Waiting for events...");

    loop {
        // Try to receive events without blocking
        while let Ok(event) = event_rx.try_recv() {
            last_event_msg = format!("[{}] {}", event.event_type, event.data);
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Title
            let title = Paragraph::new(" SUNDAY DASHBOARD - Autonomous Agent Runtime ")
                .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
            f.render_widget(title, chunks[0]);

            // Main Content Area
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(chunks[1]);

            // Service Status
            let ports = [
                ("AI Engine (Llama)", 8081),
                ("Backend API", 8000),
                ("Frontend App", 5173),
                ("Voice Live", 8098),
                ("Discord (Rust)", 8085), // Hypothetical port for rust discord if it had one
            ];

            let items: Vec<ListItem> = ports
                .iter()
                .map(|(name, port)| {
                    let status = if check_port(*port) {
                        (" RUNNING ", Color::Green)
                    } else {
                        (" STOPPED ", Color::Red)
                    };
                    ListItem::new(format!("{:<20} : [{}]", name, status.0)).style(Style::default().fg(status.1))
                })
                .collect();

            let services = List::new(items).block(Block::default().title(" Service Status ").borders(Borders::ALL));
            f.render_widget(services, main_chunks[0]);

            // System Info
            sys.refresh_all();
            let cpu_usage = sys.global_cpu_usage();
            let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
            let mem_total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
            
            let system_info = vec![
                ListItem::new(format!("CPU Usage: {:.1}%", cpu_usage)),
                ListItem::new(format!("Memory: {:.1}GB / {:.1}GB", mem_used, mem_total)),
                ListItem::new(format!("Processes: {}", sys.processes().len())),
            ];
            let system = List::new(system_info).block(Block::default().title(" System Resources ").borders(Borders::ALL));
            f.render_widget(system, main_chunks[1]);

            // Event Log (Real-time Bus)
            let event_log = Paragraph::new(last_event_msg.as_str())
                .block(Block::default().title(" Real-time Event Bus (Brain Status) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
            f.render_widget(event_log, chunks[2]);

            // Footer
            let footer = Paragraph::new(" Press 'q' to quit | 's' to start all | 'x' to stop all ")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[3]);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
            
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
