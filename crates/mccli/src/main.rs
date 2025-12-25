use clap::Parser;

use mccli::cli::{Cli, Commands};
use mccli::commands;
use mccli::error::Result;
use tracing::{Level, debug};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .without_time()
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    debug!("Logger initialized in DEBUG mode");

    match &cli.command {
        Some(Commands::Start) => {
            commands::start::run("0.0.0.0:8080").await?;
        }
        Some(Commands::Stop) => {
            commands::stop::run("0.0.0.0:8080").await?;
        }
        Some(Commands::Status) => {
            commands::status::run("0.0.0.0:8080").await?;
        }
        Some(Commands::Op { player, op }) => {
            commands::op::run("0.0.0.0:8080", player.to_string(), *op).await?;
        }
        None => {
            println!("No se pasó ningún comando. Usa --help");
        }
    }

    Ok(())
}
