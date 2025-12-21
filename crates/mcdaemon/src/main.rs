use mcprocess::config::Config;
use std::env;
use tokio::sync::mpsc;

use crate::actor::DaemonCommand;

mod actor;
mod tcp;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = env::var("MC_CONFIG").unwrap_or_else(|_| "configs/dev.toml".to_string());
    println!("Using config file: {}", config_path);
    let cfg = Config::load(&config_path)?;
    let (tx, rx) = mpsc::channel::<DaemonCommand>(32);
    actor::spawn_actor(cfg, rx);
    tcp::server_loop("0.0.0.0:8080", tx).await?;

    Ok(())
}
