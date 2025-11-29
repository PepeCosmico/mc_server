use mcprocess::config::Config;
use mcprocess::server::{ServerProcess, ServerState};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

// Comandos internos (hilos -> actor)
pub enum DaemonCommand {
    Start(oneshot::Sender<Result<String, String>>),
    Stop(oneshot::Sender<Result<String, String>>),
    Status(oneshot::Sender<ServerState>),
}

/// Inicia el Actor en segundo plano.
/// Devuelve el JoinHandle por si queremos esperar a que termine (opcional).
pub fn spawn_actor(cfg: Config, mut rx: mpsc::Receiver<DaemonCommand>) {
    tokio::spawn(async move {
        println!("🤖 Actor iniciado. Listo para recibir comandos.");
        let mut srv = ServerProcess::new(cfg);

        while let Some(msg) = rx.recv().await {
            match msg {
                DaemonCommand::Start(reply) => {
                    // 1. Intentamos arrancar el proceso
                    match srv.start().await {
                        Err(e) => {
                            // Si falla al crear el proceso (ej: no hay java), respondemos error inmediato
                            let _ = reply.send(Err(e.to_string()));
                        }
                        Ok(_) => {
                            // 2. El proceso arrancó, ahora hay que ESPERAR a que cargue.
                            // Obtenemos un "vigilante" del estado.
                            let mut state_rx = srv.state();

                            // 3. Lanzamos una tarea independiente para no bloquear al Actor
                            // Movemos el 'reply' (el canal de respuesta) dentro de esta tarea.
                            tokio::spawn(async move {
                                // Definimos un tiempo límite (ej: 2 minutos) para que no espere eterno
                                let wait_result = timeout(Duration::from_secs(120), async {
                                    loop {
                                        // Chequeamos el estado actual
                                        let current = *state_rx.borrow();

                                        if current == ServerState::Running {
                                            return Ok("Server running".to_string());
                                        }

                                        // Si crashea o se para mientras arranca, es un error
                                        if matches!(
                                            current,
                                            ServerState::Crashed | ServerState::Stopped
                                        ) {
                                            return Err(
                                                "Server stopped or crashed during start up process"
                                                    .to_string(),
                                            );
                                        }

                                        // Esperamos al siguiente cambio
                                        if state_rx.changed().await.is_err() {
                                            return Err("Internal error".to_string());
                                        }
                                    }
                                })
                                .await;

                                // 4. Procesamos el resultado de la espera
                                let final_response = match wait_result {
                                    Ok(Ok(msg)) => Ok(msg.to_string()), // Llegó a Running
                                    Ok(Err(e)) => Err(e.to_string()),   // Crasheó
                                    Err(_) => {
                                        Err("Timeout: El servidor tardó demasiado en arrancar."
                                            .to_string())
                                    }
                                };

                                // 5. Enviamos la respuesta al cliente TCP (que ha estado esperando todo este tiempo)
                                let _ = reply.send(final_response);
                            });
                        }
                    }
                }
                DaemonCommand::Stop(reply) => {
                    // 1. Verificamos si ya está parado para responder rápido
                    let current_state = *srv.state().borrow();
                    if matches!(current_state, ServerState::Stopped | ServerState::Crashed) {
                        let _ = reply.send(Ok("El servidor ya estaba detenido.".to_string()));
                        continue;
                    }

                    // 2. Enviamos el comando "/stop" al proceso
                    // Usamos exec_command en lugar de srv.stop() porque srv.stop() bloquea
                    // esperando el resultado, y aquí no queremos bloquear al Actor.
                    match srv.exec_command("stop").await {
                        Err(e) => {
                            let _ = reply.send(Err(e.to_string()));
                        }
                        Ok(_) => {
                            // 3. Delegamos la espera a una tarea de fondo
                            let mut state_rx = srv.state();
                            tokio::spawn(async move {
                                let wait_result = timeout(Duration::from_secs(60), async {
                                    loop {
                                        let current = *state_rx.borrow();

                                        if current == ServerState::Stopped {
                                            return Ok("Server stopped".to_string());
                                        }

                                        if matches!(
                                            current,
                                            ServerState::Crashed | ServerState::Stopped
                                        ) {
                                            return Err(
                                                "Server stopped or crashed during start up process"
                                                    .to_string(),
                                            );
                                        }

                                        if state_rx.changed().await.is_err() {
                                            return Err("Internal error".to_string());
                                        }
                                    }
                                })
                                .await;

                                // 4. Procesamos el resultado de la espera
                                let final_response = match wait_result {
                                    Ok(Ok(msg)) => Ok(msg.to_string()), // Llegó a Running
                                    Ok(Err(e)) => Err(e.to_string()),   // Crasheó
                                    Err(_) => Err(
                                        "Timeout: El servidor tardó demasiado en para el servidor."
                                            .to_string(),
                                    ),
                                };

                                // 5. Enviamos la respuesta al cliente TCP (que ha estado esperando todo este tiempo)
                                let _ = reply.send(final_response);
                            });
                        }
                    }
                }
                DaemonCommand::Status(reply) => {
                    let state = *srv.state().borrow();
                    let _ = reply.send(state);
                }
            }
        }
        println!("🤖 Actor detenido (Canal cerrado).");
    });
}
