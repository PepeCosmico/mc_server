use mcprocess::config::Config;
use tokio::sync::mpsc;

mod actor;
mod protocol;
mod tcp;

use crate::actor::DaemonCommand;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load("mcprocess.toml")?;
    let (tx, rx) = mpsc::channel::<DaemonCommand>(32);
    actor::spawn_actor(cfg, rx);
    tcp::server_loop("0.0.0.0:8080", tx).await?;

    Ok(())
}
