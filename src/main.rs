use mcprocess::{config::Config, server::ServerProcess, ServerState};
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = Config::load("mcprocess.toml")?;
    let mut srv = ServerProcess::new(cfg);

    let mut state_rx = srv.state();
    let mut logs_rx = srv.logs();

    // Imprime los logs en pantalla
    tokio::spawn(async move {
        while let Ok(line) = logs_rx.recv().await {
            println!("[MC] {line}");
        }
    });

    srv.start().await?;
    println!("server start requested");

    loop {
        state_rx.changed().await?;
        println!("state: {:?}", *state_rx.borrow());
        if *state_rx.borrow() == ServerState::Running {
            break;
        }
        if *state_rx.borrow() == ServerState::Crashed {
            eprintln!("Server crashed during startup");
            return Ok(());
        }
    }

    println!("server running. Sending command 'list'...");
    srv.exec_command("list").await?;
    println!("server running. Sending command save");
    srv.exec_command("save-all").await?;


    sleep(Duration::from_secs(10)).await;

    println!("Stopping...");
    srv.stop(20).await?;

    Ok(())
}
