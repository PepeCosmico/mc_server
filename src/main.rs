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


    println!("--- SERVIDOR ONLINE ---");

    println!("\n📊 [MONITOR] Iniciando monitorización de recursos (10 segundos)...");

    // BUCLE DE MONITORIZACIÓN
    for i in 1..=10 {
        sleep(Duration::from_secs(1)).await;

        // Llamamos a tu nueva función
        match srv.get_metrics() {
            Some((cpu, mem)) => {
                // Convertimos bytes a Megabytes para que sea legible
                let mem_mb = mem as f64 / 1024.0 / 1024.0;

                println!(
                    "⏱️  Seg {} | CPU: {:.2}% | RAM: {:.2} MB",
                    i, cpu, mem_mb
                );
            }
            None => eprintln!("⚠️ No se pudieron leer las métricas (¿Proceso muerto?)"),
        }

        // Si el servidor crashea mientras medimos, salimos
        if *srv.state().borrow() == ServerState::Crashed {
            println!("❌ El servidor murió durante la medición.");
            break;
        }
    }

    // 8. PRUEBA 4: Parada
    println!("> Enviando /stop...");
    srv.stop(30).await?;

    println!("👋 Programa finalizado.");
    Ok(())
}