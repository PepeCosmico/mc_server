use mcprocess::{config::{BackupCfg, Config, JavaCfg, ServerCfg}, server::{ServerProcess, ServerState}};
use std::{fs::File, time::Duration};
use tempfile::tempdir;
use tokio::time::timeout;

// --- Helpers ---

fn get_mock_script_path() -> String {
    let root = std::env::current_dir().expect("Failed to get current dir");
    let path = root.join("tests/resources/mock_java.sh");
    if !path.exists() { panic!("Mock script not found at {:?}", path); }
    path.to_str().unwrap().to_string()
}

fn create_full_config(work_dir: &str, backup_dir: &str) -> Config {
    Config {
        java: JavaCfg {
            path: get_mock_script_path(), // Ejecutamos el script bash
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
            path: backup_dir.to_string(),
        },
    }
}

// --- TESTS ---

#[tokio::test]
async fn test_full_lifecycle_and_backup() -> anyhow::Result<()> {
    // 1. Setup: Directorios temporales
    let temp_root = tempdir()?;
    let work_path = temp_root.path().join("server");
    let backup_path = temp_root.path().join("backups");

    // Creamos estructura de archivos para que el backup tenga algo que comprimir
    std::fs::create_dir_all(&work_path)?;
    File::create(work_path.join("server.jar"))?; // Fake jar
    File::create(work_path.join("world_data.txt"))?; // Archivo dummy para backup

    // 2. Configuración
    let cfg = create_full_config(
        work_path.to_str().unwrap(),
        backup_path.to_str().unwrap(),
    );
    let mut srv = ServerProcess::new(cfg);
    let mut state_rx = srv.state();

    // 3. Start
    srv.start().await?;

    // Esperar a Running (Timeout 5s)
    timeout(Duration::from_secs(5), async {
        while *state_rx.borrow() != ServerState::Running {
            state_rx.changed().await?;
        }
        Ok::<_, anyhow::Error>(())
    }).await??;

    assert_eq!(*state_rx.borrow(), ServerState::Running);

    // 4. TEST BACKUP (Aquí probamos la lógica de Save-All y Notify)
    println!("Iniciando test de backup...");
    let backup_result = srv.backup().await;

    assert!(backup_result.is_ok(), "Backup falló: {:?}", backup_result.err());
    let backup_filename = backup_result?;

    // Verificar que el archivo existe
    let expected_file = backup_path.join(backup_filename);
    assert!(expected_file.exists(), "El archivo .tar.gz no se creó");

    // Verificar que el estado volvió a Running después del backup
    assert_eq!(*state_rx.borrow(), ServerState::Running);

    // 5. Stop Suave
    srv.stop(5).await?;
    assert_eq!(*state_rx.borrow(), ServerState::Stopped);

    Ok(())
}

#[tokio::test]
async fn test_crash_detection() -> anyhow::Result<()> {
    let temp_root = tempdir()?;
    let work_path = temp_root.path().join("server");
    std::fs::create_dir_all(&work_path)?;
    File::create(work_path.join("server.jar"))?;

    let cfg = create_full_config(work_path.to_str().unwrap(), "");
    let mut srv = ServerProcess::new(cfg);
    let mut state_rx = srv.state();

    srv.start().await?;

    // Esperar arranque
    timeout(Duration::from_secs(5), async {
        while *state_rx.borrow() != ServerState::Running { state_rx.changed().await?; }
        Ok::<_, anyhow::Error>(())
    }).await??;

    // 6. Simular CRASH enviando el comando "crash" al mock
    srv.exec_command("crash").await?;

    // Esperar a que el reaper detecte la muerte
    timeout(Duration::from_secs(2), async {
        loop {
            let s = *state_rx.borrow();
            if s == ServerState::Crashed || s == ServerState::Stopped {
                return Ok::<_, anyhow::Error>(s);
            }
            state_rx.changed().await?;
        }
    }).await??;

    // El mock sale con exit code 1, así que debería ser Crashed (si tu reaper detecta exit codes)
    // O Stopped si solo detecta cierre. En tu código actual detecta status.success().
    // Como mock "exit 1", debería ser Crashed.
    let final_state = *state_rx.borrow();
    assert_eq!(final_state, ServerState::Crashed);

    Ok(())
}