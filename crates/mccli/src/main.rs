use std::time::Duration;

use clap::Parser;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use mccli::{
    cli::{Cli, Commands},
    commands,
    config::AppConfig,
    error::Result,
};
use tracing::{Level, debug, error};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::WARN
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .without_time()
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    debug!("Logger initialized in DEBUG mode");

    let cfg = AppConfig::load()?;

    let addr = cfg.get_address();
    match &cli.command {
        Some(Commands::Start) => {
            commands::start::run(&addr).await?;
        }
        Some(Commands::Stop) => {
            commands::stop::run(&addr).await?;
        }
        Some(Commands::Status) => {
            commands::status::run(&addr).await?;
        }
        Some(Commands::Op { player, op }) => {
            commands::op::run(&addr, player.to_string(), *op).await?;
        }
        None => {
            error!("Not a valid command. Use --help")
        }
    }

    Ok(())
}
