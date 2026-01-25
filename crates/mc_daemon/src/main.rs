use mc_config::McConfig;
use tokio::sync::mpsc;

use mc_daemon::{actor, actor::DaemonCommand, tcp};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = McConfig::new()?;

    let (tx, rx) = mpsc::channel::<DaemonCommand>(32);
    let addr = cfg.client.get_addr();

    actor::spawn_actor(cfg, rx);
    tcp::server_loop(&addr, tx).await?;

    Ok(())
}
