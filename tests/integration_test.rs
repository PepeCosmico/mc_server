use mcprocess::config::BackupCfg;
use mcprocess::{config::{Config, JavaCfg, ServerCfg}, server::{ServerProcess, ServerState}};
use std::{fs::File, time::Duration};
use tempfile::tempdir;
use tokio::time::timeout;

// Helper para crear config
fn create_test_config(work_dir: &str, script_path: &str, backup_path: &str) -> Config {
    Config {
        java: JavaCfg {
            path: script_path.to_string(),
            xms: "1M".to_string(),
            xmx: "1M".to_string(),
            extra_args: vec![],
        },
        server: ServerCfg {
            working_dir: work_dir.to_string(),
            jar: "server.jar".to_string(),
            nogui: true,
            auto_eula: true,
        },
        backup: BackupCfg {
            path: backup_path.to_string()
        },
    }
}

// FIX: Helper para obtener la ruta ABSOLUTA del script mock
fn get_mock_script_path() -> String {
    let root = std::env::current_dir().expect("Failed to get current dir");
    let path = root.join("tests/resources/mock_java.sh");

    if !path.exists() {
        panic!("
        ERROR CRÍTICO: No se encuentra el script de prueba.
        1. Asegúrate de crear 'tests/resources/mock_java.sh'
        2. En Linux/Mac ejecuta: chmod +x tests/resources/mock_java.sh
        Ruta buscada: {:?}
        ", path);
    }
    path.to_str().unwrap().to_string()
}

#[tokio::test]
async fn test_happy_path_lifecycle() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let dir_path = dir.path().to_str().unwrap();

    // Fake jar
    let jar_path = dir.path().join("server.jar");
    File::create(jar_path)?;

    // FIX: Usamos ruta absoluta
    let script_path = get_mock_script_path();
    let cfg = create_test_config(dir_path, &script_path);

    let mut srv = ServerProcess::new(cfg);
    let mut state_rx = srv.state();
    let _logs_rx = srv.logs();

    // Start
    srv.start().await?;

    // Esperar a Running
    timeout(Duration::from_secs(3), async {
        loop {
            if *state_rx.borrow() == ServerState::Running { break; }
            if state_rx.changed().await.is_err() { break; }
        }
    }).await.expect("Timed out waiting for Running state");

    assert_eq!(*state_rx.borrow(), ServerState::Running);

    // Comandos
    srv.exec_command("say Hello").await?;

    // Stop
    srv.stop(5).await?;

    assert_eq!(*state_rx.borrow(), ServerState::Stopped);

    Ok(())
}

#[tokio::test]
async fn test_missing_jar_error() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let dir_path = dir.path().to_str().unwrap();
    // No creamos jar

    let cfg = create_test_config(dir_path, "java");
    let mut srv = ServerProcess::new(cfg);

    let result = srv.start().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_force_kill_timeout() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let dir_path = dir.path().to_str().unwrap();

    File::create(dir.path().join("server.jar"))?;

    // Script "malo" (bucle infinito)
    let bad_script_path = dir.path().join("bad_server.sh");
    tokio::fs::write(&bad_script_path, r#"#!/bin/sh
        echo "[INFO] Done (1.0s)!"
        while true; do sleep 1; done
    "#).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bad_script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bad_script_path, perms)?;
    }

    // FIX: Pasar ruta absoluta del bad script también, por si acaso
    let bad_script_abs = bad_script_path.canonicalize()?;
    let cfg = create_test_config(dir_path, bad_script_abs.to_str().unwrap());

    let mut srv = ServerProcess::new(cfg);
    let mut state_rx = srv.state();

    srv.start().await?;

    // Esperar a Running
    timeout(Duration::from_secs(2), async {
        while *state_rx.borrow() != ServerState::Running {
            state_rx.changed().await?;
        }
        Ok::<_, anyhow::Error>(())
    }).await??;

    println!("Intentando stop con timeout corto...");
    srv.stop(1).await?;

    // FIX: Esperar a que el estado se actualice post-kill
    // Le damos 1 segundo al reaper para detectar la muerte y actualizar el canal
    let final_state = timeout(Duration::from_secs(1), async {
        loop {
            let s: ServerState = *state_rx.borrow_and_update();
            if matches!(s, ServerState::Stopped | ServerState::Crashed) {
                return s;
            }
            state_rx.changed().await.unwrap();
        }
    }).await;

    // Verificamos que finalizó correctamente
    match final_state {
        Ok(state) => assert!(matches!(state, ServerState::Stopped | ServerState::Crashed)),
        Err(_) => panic!("El estado no cambió a Stopped/Crashed después del kill"),
    }

    Ok(())
}