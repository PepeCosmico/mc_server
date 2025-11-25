use mcprocess::logs::McLog;
use mcprocess::{config::Config, logs::ServerEvent, server::ServerProcess, server::ServerState};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Cargar configuración
    let cfg = Config::load("mcprocess.toml")?;
    let mut srv = ServerProcess::new(cfg);

    // 2. Suscribirnos a los logs y al estado
    let mut logs_rx = srv.logs();
    let mut state_rx = srv.state();

    // --- TAREA DE MONITOREO DE LOGS ---
    tokio::spawn(async move {
        while let Ok(json_line) = logs_rx.recv().await {
            if let Ok(entry) = serde_json::from_str::<McLog>(&json_line) {
                match entry.event {
                    ServerEvent::Ready(time) => println!("✅ [EVENTO] SERVIDOR LISTO (Tardó: {})", time),
                    ServerEvent::Joined(player) => println!("👋 [EVENTO] Jugador conectado: {}", player),
                    ServerEvent::Chat { author, msg } => println!("💬 [CHAT] {}: {}", author, msg),
                    ServerEvent::Saving => println!("💾 [EVENTO] El servidor comenzó a guardar..."),
                    ServerEvent::Saved => println!("✅ [EVENTO] El servidor terminó de guardar."),
                    ServerEvent::Stopping => println!("🛑 [EVENTO] El servidor se está apagando..."),
                    _ => {} // Ignoramos logs normales para limpiar consola
                }
            }
        }
    });

    // --- TAREA DE MONITOREO DE ESTADO (NUEVO) ---
    // Esto es vital para ver si tu lógica de backup cambia los estados correctamente
    // Deberías ver: Running -> Saving -> Running
    tokio::spawn(async move {
        while state_rx.changed().await.is_ok() {
            let state = *state_rx.borrow();
            println!("🔄 [ESTADO] Cambio detectado: {:?}", state);
        }
    });

    // 3. Arrancar Servidor
    println!("🚀 Iniciando servidor...");
    srv.start().await?;

    // 4. Esperar a que esté Running
    loop {
        // Usamos un pequeño sleep para no saturar comprobando
        sleep(Duration::from_millis(100)).await;
        // Forma segura de chequear el estado actual
        if *srv.state().borrow() == ServerState::Running {
            break;
        }
    }
    println!("--- SERVIDOR OPERATIVO Y ESPERANDO COMANDOS ---");

    // 5. PRUEBA 1: Comando de chat / say
    println!("> Enviando /say...");
    srv.exec_command("say Probando el sistema de logs...").await?;
    sleep(Duration::from_secs(2)).await;

    // 6. PRUEBA 2: Comando de guardado manual
    println!("> Enviando /save-all (Manual)...");
    srv.exec_command("save-all").await?;
    sleep(Duration::from_secs(3)).await;

    // 7. PRUEBA 3: Listar jugadores
    println!("> Enviando /list...");
    srv.exec_command("list").await?;
    sleep(Duration::from_secs(2)).await;

    // --- NUEVO: PRUEBA DE BACKUP ---
    println!("\n📦 [TEST] Iniciando Backup Automático...");
    println!("   (Deberías ver el estado cambiar a Saving y luego a Running)");

    // Esta función bloqueará este hilo hasta que termine el backup,
    // pero el servidor seguirá respondiendo en segundo plano.
    match srv.backup().await {
        Ok(filename) => {
            println!("🎉 [TEST] Backup completado exitosamente: {}", filename);
        }
        Err(e) => {
            eprintln!("❌ [TEST] Falló el backup: {}", e);
        }
    }
    println!("---------------------------------------------------\n");

    sleep(Duration::from_secs(2)).await;

    // 8. PRUEBA 4: Parada
    println!("> Enviando /stop...");
    srv.stop(30).await?;

    println!("👋 Programa finalizado.");
    Ok(())
}