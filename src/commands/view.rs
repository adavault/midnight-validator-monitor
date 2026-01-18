//! View command - interactive TUI for real-time monitoring

use crate::db::Database;
use crate::metrics::MetricsClient;
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
    #[arg(short, long)]
    pub rpc_url: Option<String>,

    /// SQLite database path
    #[arg(short, long)]
    pub db_path: Option<PathBuf>,

    /// Refresh interval in milliseconds
    #[arg(long)]
    pub refresh_interval: Option<u64>,
}

/// Run the view command
pub async fn run(args: ViewArgs) -> Result<()> {
    // Load configuration
    let config = crate::config::Config::load()?;

    // Use args or fall back to config
    let rpc_url = args.rpc_url.unwrap_or(config.rpc.url);
    let db_path = args.db_path.unwrap_or_else(|| std::path::PathBuf::from(&config.database.path));
    let refresh_interval = args.refresh_interval.unwrap_or(config.view.refresh_interval_ms);

    // Connect to RPC, metrics, and database BEFORE initializing terminal
    let rpc = RpcClient::with_timeout(&rpc_url, config.rpc.timeout_ms);
    let metrics = MetricsClient::new(&config.rpc.metrics_url);
    let db = Database::open(&db_path)
        .context(format!("Failed to open database at {}.

Tip: If you installed MVM, the database should be at /opt/midnight/mvm/data/mvm.db
     Try: mvm view --db-path /opt/midnight/mvm/data/mvm.db
     Or set MVM_DB_PATH=/opt/midnight/mvm/data/mvm.db in your environment

     If running locally without install, use: mvm view --db-path ./mvm.db", db_path.display()))?;

    // Initialize terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to create terminal")?;

    // Initialize app with network-specific timing
    let mut app = App::new().with_chain_timing(config.chain.timing());

    // Set node name from config if specified
    if let Some(ref name) = config.validator.name {
        app.state.node_name = name.clone();
    }

    // Set expected IP for external address filtering
    if let Some(ref ip) = config.view.expected_ip {
        app.expected_ip = Some(ip.clone());
    }

    // Do initial update
    if let Err(e) = app.update(&rpc, &metrics, &db).await {
        error!("Initial update failed: {}", e);
    }

    // Create event handler with 1-second tick for UI updates
    let event_handler = EventHandler::new(Duration::from_millis(1000));

    // Run the TUI loop (data refresh at configured interval)
    let res = run_tui(&mut terminal, &mut app, &rpc, &metrics, &db, &event_handler, refresh_interval).await;

    // Restore terminal (always, even on error)
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    // Check for errors
    if let Err(err) = res {
        eprintln!("Error in TUI: {}", err);
        return Err(err);
    }

    Ok(())
}

async fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rpc: &RpcClient,
    metrics: &MetricsClient,
    db: &Database,
    event_handler: &EventHandler,
    data_refresh_interval_ms: u64,
) -> Result<()> {
    let data_refresh_interval = Duration::from_millis(data_refresh_interval_ms);

    loop {
        // Render UI (every tick - 1 second)
        terminal.draw(|f| crate::tui::render(f, app))?;

        // Handle events
        match event_handler.next()? {
            Event::Key(key) => {
                if !crate::tui::event::handle_key_event(key, app) {
                    break;
                }
            }
            Event::Tick | Event::Resize => {
                // Only fetch new data at the configured refresh interval
                if app.last_update.elapsed() >= data_refresh_interval {
                    if let Err(e) = app.update(rpc, metrics, db).await {
                        error!("Update failed: {}", e);
                    }
                }
                // UI still redraws every tick to update the "Updated Xs ago" counter
            }
        }

        // Check if should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
