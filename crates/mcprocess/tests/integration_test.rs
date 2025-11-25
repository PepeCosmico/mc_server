use mcprocess::{config::{BackupCfg, Config, JavaCfg, ServerCfg}, server::{ServerProcess, ServerState}};
use std::{fs::File, time::Duration};
use tempfile::tempdir;
use tokio::time::timeout;

// --- Helpers ---

fn get_mock_script_path() -> String {
    let current_dir = std::env::current_dir().expect("Failed to get current dir");

    // Intentamos buscar la ruta relativa dependiendo de dónde se lance el test
    let possible_paths = vec![
        // Opción A: Ejecutando desde crates/mcprocess/
        current_dir.join("tests/resources/mock_java.sh"),
        // Opción B: Ejecutando desde el root del workspace (mc_server/)
        current_dir.join("crates/mcprocess/tests/resources/mock_java.sh"),
    ];

    for path in possible_paths {
        if path.exists() {
            // Encontrado! Devolvemos ruta absoluta
            return path.canonicalize().unwrap().to_str().unwrap().to_string();
        }
    }

    panic!("
    ERROR: No se encuentra 'mock_java.sh'.
    Buscado en:
    - crates/mcprocess/tests/resources/mock_java.sh
    - tests/resources/mock_java.sh

    Directorio actual: {:?}
    ", current_dir);
}

fn create_full_config(work_dir: &str, backup_dir: &str) -> Config {
    Config {
        java: JavaCfg {
            path: get_mock_script_path(),
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
    }).await.map_err(|_| anyhow::anyhow!("Timeout esperando estado {:?}", target))?
}

// --- TESTS ---

#[tokio::test]
async fn test_full_lifecycle_and_backup() -> anyhow::Result<()> {
    let temp_root = tempdir()?;
    let work_path = temp_root.path().join("server");
    let backup_path = temp_root.path().join("backups");

    std::fs::create_dir_all(&work_path)?;
    File::create(work_path.join("server.jar"))?;
    File::create(work_path.join("world_data.txt"))?;

    let cfg = create_full_config(work_path.to_str().unwrap(), backup_path.to_str().unwrap());
    let mut srv = ServerProcess::new(cfg);

    // 1. Start
    srv.start().await?;
    wait_for_state(&srv, ServerState::Running).await?;

    // 2. Backup
    println!("Iniciando backup...");
    // Esto enviará "save-all" al mock. El mock debe responder "Saved the game"
    // Si el mock no responde eso, backup() fallará por timeout interno (si lo programaste) o se bloqueará.
    // Asegúrate de que tu función backup() en server.rs tenga un timeout interno.
    let backup_result = srv.backup().await;

    assert!(backup_result.is_ok(), "Error en backup: {:?}", backup_result.err());
    let filename = backup_result?;

    assert!(backup_path.join(filename).exists());
    assert_eq!(*srv.state().borrow(), ServerState::Running);

    // 3. Stop
    srv.stop(5).await?;
    wait_for_state(&srv, ServerState::Stopped).await?;

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

    srv.start().await?;
    wait_for_state(&srv, ServerState::Running).await?;

    // Enviamos crash
    srv.exec_command("crash").await?;

    // Esperamos a que muera (Stopped o Crashed)
    // Usamos el helper wait_for_state modificado o un loop manual con timeout
    let mut state_rx = srv.state();
    timeout(Duration::from_secs(3), async {
        loop {
            let s = *state_rx.borrow();
            if s == ServerState::Crashed || s == ServerState::Stopped {
                break;
            }
            if state_rx.changed().await.is_err() { break; }
        }
    }).await?;

    let final_s = *srv.state().borrow();
    assert!(matches!(final_s, ServerState::Crashed | ServerState::Stopped));

    Ok(())
}

#[tokio::test]
async fn test_metrics_collection() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let dir = tmp.path();
    File::create(dir.join("server.jar"))?;

    let cfg = create_full_config(dir.to_str().unwrap(), "backups"); // Ruta backup dummy
    let mut srv = ServerProcess::new(cfg);

    srv.start().await?;

    // AQUÍ estaba el bloqueo original.
    // Si el server no arrancaba, wait_for_state esperaba infinito.
    // Ahora wait_for_state tiene timeout interno y lanzará error si falla.
    wait_for_state(&srv, ServerState::Running).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Métricas
    let m1 = srv.get_metrics();
    assert!(m1.is_some(), "Métricas devolvieron None");

    tokio::time::sleep(Duration::from_millis(200)).await;

    let m2 = srv.get_metrics();
    assert!(m2.is_some());
    let (cpu, mem) = m2.unwrap();

    println!("Metrics: CPU {}%, MEM {} bytes", cpu, mem);
    assert!(mem > 0);
    assert!(cpu >= 0.0);

    srv.stop(5).await?;
    Ok(())
}