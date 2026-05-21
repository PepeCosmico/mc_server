use mc_config::McConfig;
use mc_process::server::ServerProcess;
use mc_types::server::state::ServerState;
use std::path::PathBuf;
use std::{fs::File, time::Duration};
use tempfile::tempdir;
use tokio::time::timeout;
// --- Helpers ---

fn create_full_config(work_dir: &str) -> McConfig {
    let mut cfg = McConfig::default();
    cfg.server.working_dir = PathBuf::from(work_dir);
    cfg.java.path = env!("CARGO_BIN_EXE_mock_java").to_string();
    cfg
}

// MEJORA: Este helper ahora usa timeout y suscripción a eventos
async fn wait_for_state(srv: &ServerProcess, target: ServerState) -> anyhow::Result<()> {
    let mut state_rx = srv.state();

    // Si ya estamos en el estado, salimos
    if *state_rx.borrow() == target {
        return Ok(());
    }

    // Esperamos máximo 5 segundos
    timeout(Duration::from_secs(5), async {
        loop {
            state_rx.changed().await?;
            if *state_rx.borrow() == target {
                return Ok(());
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout esperando estado {:?}", target))?
}

// --- TESTS ---

#[tokio::test]
async fn test_start_version_detection() -> anyhow::Result<()> {
    let temp_root = tempdir()?;
    let work_path = temp_root.path().join("server");
    std::fs::create_dir_all(&work_path)?;

    File::create(work_path.join("server.jar"))?;

    let cfg = create_full_config(work_path.to_str().unwrap());
    let mut srv = ServerProcess::new(cfg);

    srv.start().await?;
    wait_for_state(&srv, ServerState::Running).await?;

    println!("✅ Server Started");

    assert!(srv.version().borrow().is_some());
    assert_eq!(
        srv.version().borrow().as_ref().unwrap().clone().mc_version,
        String::from("1.21.4")
    );
    Ok(())
}

#[tokio::test]
async fn test_crash_detection() -> anyhow::Result<()> {
    let temp_root = tempdir()?;
    let work_path = temp_root.path().join("server");
    std::fs::create_dir_all(&work_path)?;
    File::create(work_path.join("server.jar"))?;

    let cfg = create_full_config(work_path.to_str().unwrap());
    let mut srv = ServerProcess::new(cfg);

    srv.start().await?;
    wait_for_state(&srv, ServerState::Running).await?;

    srv.exec_command("crash").await?;

    let mut state_rx = srv.state();
    timeout(Duration::from_secs(3), async {
        loop {
            let s = *state_rx.borrow();
            if s == ServerState::Crashed || s == ServerState::Stopped {
                break;
            }
            if state_rx.changed().await.is_err() {
                break;
            }
        }
    })
    .await?;

    let final_s = *srv.state().borrow();
    assert!(matches!(
        final_s,
        ServerState::Crashed | ServerState::Stopped
    ));

    Ok(())
}

#[tokio::test]
async fn test_metrics_collection() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let dir = tmp.path();
    File::create(dir.join("server.jar"))?;

    let cfg = create_full_config(dir.to_str().unwrap());
    let mut srv = ServerProcess::new(cfg);

    srv.start().await?;
    wait_for_state(&srv, ServerState::Running).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let m1 = srv.get_metrics();
    assert!(m1.is_some(), "Métricas devolvieron None");

    tokio::time::sleep(Duration::from_millis(200)).await;

    let m2 = srv.get_metrics();
    assert!(m2.is_some());
    let (cpu, mem) = m2.unwrap();

    println!("Metrics: CPU {}%, MEM {} bytes", cpu, mem);
    assert!(mem > 0);
    assert!(cpu >= 0.0);

    srv.stop().await?;
    Ok(())
}
