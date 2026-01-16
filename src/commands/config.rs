//! Configuration management command

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show current configuration (after applying all overrides)
    Show,

    /// Validate configuration file
    Validate,

    /// Print example configuration file
    Example,

    /// Show configuration file search paths
    Paths,
}

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Show => run_show().await?,
        ConfigCommands::Validate => run_validate().await?,
        ConfigCommands::Example => run_example().await?,
        ConfigCommands::Paths => run_paths().await?,
    }

    Ok(())
}

async fn run_show() -> Result<()> {
    let config = crate::config::Config::load()?;
    config.validate()?;

    println!("Current Configuration:");
    println!("=====================\n");

    let toml_str = toml::to_string_pretty(&config)?;
    println!("{}", toml_str);

    println!("\nConfiguration loaded successfully.");
    println!("Priority: CLI flags > Environment variables > Config file > Defaults");

    Ok(())
}

async fn run_validate() -> Result<()> {
    println!("Validating configuration...\n");

    let paths = crate::config::Config::config_file_paths();
    let mut found = false;

    for path in &paths {
        if path.exists() {
            found = true;
            println!("Found config file: {}", path.display());

            match crate::config::Config::load() {
                Ok(config) => {
                    match config.validate() {
                        Ok(_) => {
                            println!("✓ Configuration is valid");
                        }
                        Err(e) => {
                            println!("✗ Configuration validation failed: {}", e);
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to load configuration: {}", e);
                    return Err(e);
                }
            }
            break;
        }
    }

    if !found {
        println!("No configuration file found. Using defaults.");
        let config = crate::config::Config::default();
        config.validate()?;
        println!("✓ Default configuration is valid");
    }

    Ok(())
}

async fn run_example() -> Result<()> {
    println!("# Midnight Validator Monitor Configuration File");
    println!("#");
    println!("# This file can be placed in one of these locations (in order of priority):");
    println!("# 1. ./mvm.toml (current directory)");
    println!("# 2. ~/.config/mvm/config.toml (user config directory)");
    println!("# 3. /etc/mvm/config.toml (system config directory)");
    println!("#");
    println!("# Configuration priority: CLI flags > Environment variables > Config file > Defaults");
    println!();

    println!("{}", crate::config::Config::example_toml());

    Ok(())
}

async fn run_paths() -> Result<()> {
    println!("Configuration File Search Paths:");
    println!("================================\n");

    let paths = crate::config::Config::config_file_paths();

    for (i, path) in paths.iter().enumerate() {
        let exists = if path.exists() { "✓ EXISTS" } else { "  " };
        println!("{}. {} {}", i + 1, path.display(), exists);
    }

    println!("\nConfiguration files are searched in order from top to bottom.");
    println!("The first file found will be used.");

    Ok(())
}
