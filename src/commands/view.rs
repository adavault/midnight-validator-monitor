//! View command - interactive TUI for real-time monitoring

use crate::db::Database;
use crate::rpc::RpcClient;
use crate::tui::{App, Event, EventHandler};
use anyhow::{Context, Result};
use clap::Args;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tracing::error;

/// View command arguments
#[derive(Args, Debug)]
pub struct ViewArgs {
    /// Validator node RPC endpoint URL
    #[arg(short, long, default_value = "http://localhost:9944")]
    pub rpc_url: String,

    /// SQLite database path
    #[arg(short, long, default_value = "./mvm.db")]
    pub db_path: PathBuf,

    /// Refresh interval in milliseconds
    #[arg(long, default_value_t = 2000)]
    pub refresh_interval: u64,
}

/// Run the view command
pub async fn run(args: ViewArgs) -> Result<()> {
    // Initialize terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Initialize app
    let mut app = App::new();

    // Connect to RPC and database
    let rpc = RpcClient::new(&args.rpc_url);
    let db = Database::open(&args.db_path)?;

    // Do initial update
    if let Err(e) = app.update(&rpc, &db).await {
        error!("Initial update failed: {}", e);
    }

    // Create event handler
    let event_handler = EventHandler::new(Duration::from_millis(args.refresh_interval));

    // Run the TUI loop
    let res = run_tui(&mut terminal, &mut app, &rpc, &db, &event_handler).await;

    // Restore terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;

    // Check for errors
    if let Err(err) = res {
        error!("Error in TUI: {}", err);
        return Err(err);
    }

    Ok(())
}

async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rpc: &RpcClient,
    db: &Database,
    event_handler: &EventHandler,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|f| crate::tui::render(f, app))?;

        // Handle events
        match event_handler.next()? {
            Event::Key(key) => {
                if !crate::tui::event::handle_key_event(key, app) {
                    break;
                }
            }
            Event::Tick | Event::Resize => {
                // Update data on tick
                if let Err(e) = app.update(rpc, db).await {
                    error!("Update failed: {}", e);
                }
            }
        }

        // Check if should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
