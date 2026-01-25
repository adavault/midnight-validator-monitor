//! Install and uninstall commands for MVM
//!
//! Provides self-installation capability so users can download just the binary
//! and run `sudo mvm install` to set up everything.

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

const INSTALL_BASE: &str = "/opt/midnight/mvm";
const BIN_DIR: &str = "/opt/midnight/mvm/bin";
const DATA_DIR: &str = "/opt/midnight/mvm/data";
const CONFIG_DIR: &str = "/opt/midnight/mvm/config";
const SYSTEMD_DIR: &str = "/etc/systemd/system";
const SYMLINK_PATH: &str = "/usr/local/bin/mvm";

#[derive(Args, Debug)]
pub struct InstallArgs {
    #[command(subcommand)]
    pub command: Option<InstallCommands>,
}

#[derive(Subcommand, Debug)]
pub enum InstallCommands {
    /// Uninstall MVM (removes services and symlink, optionally data)
    Uninstall {
        /// Also remove data directory (database, config)
        #[arg(long)]
        remove_data: bool,
    },
}

pub async fn run(args: InstallArgs) -> Result<()> {
    match args.command {
        None => run_install().await,
        Some(InstallCommands::Uninstall { remove_data }) => run_uninstall(remove_data).await,
    }
}

async fn run_install() -> Result<()> {
    println!();
    println!("Midnight Validator Monitor - Installation");
    println!("==========================================");
    println!();

    // Check if running as root
    if !is_root() {
        bail!("This command must be run with sudo: sudo mvm install");
    }

    // Get the real user (when running via sudo)
    let real_user = get_real_user()?;
    println!("Installing to: {}", INSTALL_BASE);
    println!("User: {}", real_user);
    println!();

    // Stop existing services if running (track which were running)
    let was_running = stop_existing_services()?;

    // Create directories
    create_directories(&real_user)?;

    // Install binary
    install_binary(&real_user)?;

    // Create config
    create_config(&real_user)?;

    // Install systemd services
    install_systemd_services(&real_user)?;

    // Restart services that were previously running
    restart_services(&was_running)?;

    // Show completion message
    show_completion(&real_user, &was_running);

    Ok(())
}

async fn run_uninstall(remove_data: bool) -> Result<()> {
    println!();
    println!("Midnight Validator Monitor - Uninstallation");
    println!("============================================");
    println!();

    // Check if running as root
    if !is_root() {
        bail!("This command must be run with sudo: sudo mvm install uninstall");
    }

    // Stop and disable services
    stop_and_disable_services()?;

    // Remove systemd services
    remove_systemd_services()?;

    // Remove symlink
    remove_symlink()?;

    // Remove data if requested
    if remove_data {
        println!("==> Removing data directory");
        if Path::new(INSTALL_BASE).exists() {
            fs::remove_dir_all(INSTALL_BASE).context("Failed to remove data directory")?;
            println!("    Removed {}", INSTALL_BASE);
        }
    } else {
        println!();
        println!("Data preserved at: {}", INSTALL_BASE);
        println!("To remove later: sudo rm -rf {}", INSTALL_BASE);
    }

    println!();
    println!("========================================");
    println!("Uninstallation Complete!");
    println!("========================================");
    println!();

    Ok(())
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn get_real_user() -> Result<String> {
    // Check SUDO_USER first (set when running via sudo)
    if let Ok(user) = std::env::var("SUDO_USER") {
        return Ok(user);
    }
    // Fall back to USER
    if let Ok(user) = std::env::var("USER") {
        return Ok(user);
    }
    // Last resort: whoami
    let output = Command::new("whoami")
        .output()
        .context("Failed to determine user")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Tracks which services were running before installation
#[derive(Default)]
struct RunningServices {
    mvm_sync: bool,
    mvm_status_timer: bool,
}

fn stop_existing_services() -> Result<RunningServices> {
    let mut running = RunningServices::default();

    // Check and stop mvm-sync
    if is_service_active("mvm-sync") {
        running.mvm_sync = true;
        println!("==> Stopping existing mvm-sync service");
        let _ = Command::new("systemctl")
            .args(["stop", "mvm-sync"])
            .status();
    }

    // Check and stop mvm-status.timer
    if is_service_active("mvm-status.timer") {
        running.mvm_status_timer = true;
        println!("==> Stopping existing mvm-status timer");
        let _ = Command::new("systemctl")
            .args(["stop", "mvm-status.timer"])
            .status();
    }

    Ok(running)
}

fn restart_services(running: &RunningServices) -> Result<()> {
    if running.mvm_sync {
        println!("==> Restarting mvm-sync service");
        let status = Command::new("systemctl")
            .args(["start", "mvm-sync"])
            .status()
            .context("Failed to start mvm-sync")?;
        if status.success() {
            println!("    mvm-sync started successfully");
        } else {
            println!("    Warning: mvm-sync may not have started correctly");
        }
    }

    if running.mvm_status_timer {
        println!("==> Restarting mvm-status timer");
        let status = Command::new("systemctl")
            .args(["start", "mvm-status.timer"])
            .status()
            .context("Failed to start mvm-status.timer")?;
        if status.success() {
            println!("    mvm-status.timer started successfully");
        } else {
            println!("    Warning: mvm-status.timer may not have started correctly");
        }
    }

    Ok(())
}

fn is_service_active(service: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", service])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn create_directories(user: &str) -> Result<()> {
    println!("==> Creating directories");

    for dir in &[BIN_DIR, DATA_DIR, CONFIG_DIR] {
        fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {}", dir))?;
    }

    // Set ownership
    set_ownership(INSTALL_BASE, user)?;

    println!("    Directories created at {}", INSTALL_BASE);
    Ok(())
}

fn install_binary(user: &str) -> Result<()> {
    println!("==> Installing binary");

    // Get the path to the current executable
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;

    let dest_path = format!("{}/mvm", BIN_DIR);

    // Copy the binary
    fs::copy(&current_exe, &dest_path)
        .with_context(|| format!("Failed to copy binary to {}", dest_path))?;

    // Set permissions (755)
    let mut perms = fs::metadata(&dest_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dest_path, perms)?;

    // Set ownership
    set_ownership(&dest_path, user)?;

    // Create symlink
    let symlink = Path::new(SYMLINK_PATH);
    if symlink.exists() || symlink.is_symlink() {
        fs::remove_file(symlink).ok();
    }
    std::os::unix::fs::symlink(&dest_path, symlink).context("Failed to create symlink")?;

    println!("    Binary installed to {}", dest_path);
    println!("    Symlink created at {}", SYMLINK_PATH);

    Ok(())
}

fn create_config(user: &str) -> Result<()> {
    println!("==> Creating configuration");

    let config_path = format!("{}/config.toml", CONFIG_DIR);

    if Path::new(&config_path).exists() {
        println!("    Config already exists, skipping");
        return Ok(());
    }

    let config_content = format!(
        r#"[rpc]
url = "http://localhost:9944"
metrics_url = "http://localhost:9615/metrics"
# Optional: node_exporter for system metrics
# node_exporter_url = "http://localhost:9100/metrics"

[database]
path = "{}/mvm.db"

[validator]
# keystore_path = "/path/to/your/keystore"

[sync]
batch_size = 100
poll_interval_secs = 6

[daemon]
pid_file = "{}/mvm-sync.pid"
"#,
        DATA_DIR, DATA_DIR
    );

    fs::write(&config_path, config_content).context("Failed to write config file")?;

    set_ownership(&config_path, user)?;

    println!("    Config created at {}", config_path);

    Ok(())
}

fn install_systemd_services(user: &str) -> Result<()> {
    println!("==> Installing systemd services");

    let version = env!("CARGO_PKG_VERSION");

    // mvm-sync.service
    let sync_service = format!(
        r#"[Unit]
Description=Midnight Validator Monitor v{} - Block Sync Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User={}
WorkingDirectory={}
Environment="MVM_DB_PATH={}/mvm.db"
ExecStart={}/mvm sync --daemon --pid-file {}/mvm-sync.pid
Restart=on-failure
RestartSec=10s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
"#,
        version, user, INSTALL_BASE, DATA_DIR, BIN_DIR, DATA_DIR
    );

    fs::write(format!("{}/mvm-sync.service", SYSTEMD_DIR), sync_service)
        .context("Failed to write mvm-sync.service")?;

    // mvm-status.service
    let status_service = format!(
        r#"[Unit]
Description=Midnight Validator Monitor v{} - Status Check
After=network-online.target

[Service]
Type=oneshot
User={}
WorkingDirectory={}
Environment="MVM_DB_PATH={}/mvm.db"
ExecStart={}/mvm status --once
StandardOutput=journal
StandardError=journal
"#,
        version, user, INSTALL_BASE, DATA_DIR, BIN_DIR
    );

    fs::write(
        format!("{}/mvm-status.service", SYSTEMD_DIR),
        status_service,
    )
    .context("Failed to write mvm-status.service")?;

    // mvm-status.timer
    let status_timer = r#"[Unit]
Description=Midnight Validator Monitor - Periodic Status Check

[Timer]
OnBootSec=1min
OnUnitActiveSec=5min
Persistent=true

[Install]
WantedBy=timers.target
"#;

    fs::write(format!("{}/mvm-status.timer", SYSTEMD_DIR), status_timer)
        .context("Failed to write mvm-status.timer")?;

    // Set permissions
    for file in &["mvm-sync.service", "mvm-status.service", "mvm-status.timer"] {
        let path = format!("{}/{}", SYSTEMD_DIR, file);
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms)?;
    }

    // Reload systemd
    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .context("Failed to reload systemd")?;

    println!("    Systemd services installed");

    Ok(())
}

fn stop_and_disable_services() -> Result<()> {
    println!("==> Stopping and disabling services");

    for service in &["mvm-sync", "mvm-status.timer"] {
        if is_service_active(service) {
            let _ = Command::new("systemctl").args(["stop", service]).status();
            println!("    Stopped {}", service);
        }

        // Check if enabled and disable
        let enabled = Command::new("systemctl")
            .args(["is-enabled", "--quiet", service])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if enabled {
            let _ = Command::new("systemctl")
                .args(["disable", service])
                .status();
            println!("    Disabled {}", service);
        }
    }

    Ok(())
}

fn remove_systemd_services() -> Result<()> {
    println!("==> Removing systemd services");

    for file in &["mvm-sync.service", "mvm-status.service", "mvm-status.timer"] {
        let path = format!("{}/{}", SYSTEMD_DIR, file);
        if Path::new(&path).exists() {
            fs::remove_file(&path).ok();
        }
    }

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .context("Failed to reload systemd")?;

    println!("    Systemd services removed");

    Ok(())
}

fn remove_symlink() -> Result<()> {
    let symlink = Path::new(SYMLINK_PATH);
    if symlink.exists() || symlink.is_symlink() {
        println!("==> Removing symlink");
        fs::remove_file(symlink).ok();
        println!("    Removed {}", SYMLINK_PATH);
    }
    Ok(())
}

fn set_ownership(path: &str, user: &str) -> Result<()> {
    Command::new("chown")
        .args(["-R", &format!("{}:{}", user, user), path])
        .status()
        .with_context(|| format!("Failed to set ownership on {}", path))?;
    Ok(())
}

fn show_completion(user: &str, was_running: &RunningServices) {
    println!();
    println!("========================================");
    println!("Installation Complete!");
    println!("========================================");
    println!();
    println!("Installation location: {}", INSTALL_BASE);
    println!("  Binary:    {}/mvm", BIN_DIR);
    println!("  Data:      {}", DATA_DIR);
    println!("  Config:    {}/config.toml", CONFIG_DIR);
    println!("  Database:  {}/mvm.db", DATA_DIR);
    println!();
    println!("Running as user: {}", user);

    // If services were restarted, show status
    if was_running.mvm_sync || was_running.mvm_status_timer {
        println!();
        println!("Services restarted:");
        if was_running.mvm_sync {
            println!("  - mvm-sync (was running, now restarted)");
        }
        if was_running.mvm_status_timer {
            println!("  - mvm-status.timer (was running, now restarted)");
        }
        println!();
        println!("To view logs:");
        println!("       sudo journalctl -u mvm-sync -f");
    } else {
        // Fresh install - show full next steps
        println!();
        println!("Next steps:");
        println!();
        println!("  1. Start the sync daemon:");
        println!("       sudo systemctl start mvm-sync");
        println!();
        println!("  2. Enable auto-start on boot:");
        println!("       sudo systemctl enable mvm-sync");
        println!();
        println!("  3. (Optional) Enable periodic health checks:");
        println!("       sudo systemctl enable --now mvm-status.timer");
        println!();
        println!("  4. View logs:");
        println!("       sudo journalctl -u mvm-sync -f");
        println!();
        println!("  5. Interactive TUI:");
        println!("       mvm view");
    }
    println!();
}
