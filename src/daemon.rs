//! Daemon support utilities for PID file management and signal handling

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tracing::{debug, info};

/// PID file manager
pub struct PidFile {
    path: std::path::PathBuf,
}

impl PidFile {
    /// Create a new PID file with the current process ID
    pub fn create(path: &Path) -> Result<Self> {
        let pid = std::process::id();

        // Check if PID file already exists
        if path.exists() {
            let existing_pid = fs::read_to_string(path)
                .context("Failed to read existing PID file")?
                .trim()
                .parse::<u32>()
                .ok();

            if let Some(existing_pid) = existing_pid {
                // Check if process is still running
                if process_exists(existing_pid) {
                    anyhow::bail!(
                        "PID file {} already exists with running process {}",
                        path.display(),
                        existing_pid
                    );
                }
                info!(
                    "Removing stale PID file {} (process {} not running)",
                    path.display(),
                    existing_pid
                );
                fs::remove_file(path)?;
            }
        }

        // Write PID file
        fs::write(path, pid.to_string())
            .with_context(|| format!("Failed to write PID file {}", path.display()))?;

        debug!("Created PID file {} with PID {}", path.display(), pid);

        Ok(PidFile {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Err(e) = fs::remove_file(&self.path) {
                tracing::error!("Failed to remove PID file {}: {}", self.path.display(), e);
            } else {
                debug!("Removed PID file {}", self.path.display());
            }
        }
    }
}

/// Check if a process with the given PID exists
fn process_exists(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // Send signal 0 to check if process exists
        kill(Pid::from_raw(pid as i32), Signal::SIGCHLD).is_ok()
            || kill(Pid::from_raw(pid as i32), None).is_ok()
    }

    #[cfg(not(unix))]
    {
        // On non-Unix systems, assume process might exist
        true
    }
}
