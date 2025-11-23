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
    // Aquí es donde probamos si tu parser funciona.
    // Deserializamos el JSON y hacemos match sobre el evento detectado.
    tokio::spawn(async move {
        while let Ok(json_line) = logs_rx.recv().await {
            // Intentamos convertir el string JSON de vuelta a tu struct LogEntry
            if let Ok(entry) = serde_json::from_str::<McLog>(&json_line) {
                match entry.event {
                    // Si detecta "Done!", tu parser funciona
                    ServerEvent::Ready(time) => {
                        println!("✅ [EVENTO] SERVIDOR LISTO (Tardó: {})", time);
                    }
                    // Si detecta "Joined", tu regex de login funciona
                    ServerEvent::Joined(player) => {
                        println!("👋 [EVENTO] Jugador conectado: {}", player);
                    }
                    // Si detecta chat
                    ServerEvent::Chat { author, msg } => {
                        println!("💬 [CHAT] {}: {}", author, msg);
                    }
                    // Si detecta guardado (prueba del filtro de Server Thread)
                    ServerEvent::Saving => {
                        println!("💾 [EVENTO] Guardando mundo...");
                    }
                    ServerEvent::Saved => {
                        println!("💾 [EVENTO] ¡Guardado completado!");
                    }
                    // Si detecta apagado
                    ServerEvent::Stopping => {
                        println!("🛑 [EVENTO] El servidor se está apagando...");
                    }
                    // Log normal (Unknown)
                    _ => {
                        // Imprimimos el log normal para ver qué pasa en consola
                        // print!("{}", entry.message); // Opcional, puede ser mucho ruido
                    }
                }
            }
        }
    });

    // 3. Arrancar Servidor
    println!("🚀 Iniciando servidor...");
    srv.start().await?;

    // 4. Esperar inteligentemente a que esté Running
    // Gracias a tu parser, el estado cambiará solo cuando detecte "Done"
    loop {
        state_rx.changed().await?;
        if *state_rx.borrow() == ServerState::Running {
            break;
        }
    }
    println!("--- SERVIDOR OPERATIVO Y ESPERANDO COMANDOS ---");

    // 5. PRUEBA 1: Comando de chat / say
    // Esto generará un log tipo "[Server]: Hola a todos"
    println!("> Enviando /say...");
    srv.exec_command("say Probando el sistema de logs...").await?;
    sleep(Duration::from_secs(2)).await;

    // 6. PRUEBA 2: Comando de guardado
    // Esto debería disparar el evento ServerEvent::Saving y ServerEvent::Saved
    // Si tu filtro de "Server Thread" está bien hecho, esto funcionará.
    println!("> Enviando /save-all...");
    srv.exec_command("save-all").await?;
    sleep(Duration::from_secs(3)).await;

    // 7. PRUEBA 3: Listar jugadores
    println!("> Enviando /list...");
    srv.exec_command("list").await?;
    sleep(Duration::from_secs(2)).await;

    // 8. PRUEBA 4: Parada
    // Esto debería disparar ServerEvent::Stopping
    println!("> Enviando /stop...");
    srv.stop(30).await?;

    println!("👋 Programa finalizado.");
    Ok(())
}