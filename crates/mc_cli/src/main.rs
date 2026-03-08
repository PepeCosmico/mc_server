use clap::Parser;

use mc_cli::{
    cli::{Cli, Commands},
    commands,
    error::Result,
};
use mc_config::McConfig;
use tracing::{Level, debug, error};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        debug!("Logger initialized in DEBUG mode");
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .without_time()
            .with_target(false)
            .finish();

        tracing::subscriber::set_global_default(subscriber)?;
    }

    let cfg = McConfig::new()?;

    let addr = cfg.client.get_addr();
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
