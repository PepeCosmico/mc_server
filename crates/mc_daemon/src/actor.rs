use crate::protocol::Op;
use crate::utils::wait_for_state;
use mc_config::McConfig;
use mc_process::{server::ServerProcess, state::ServerState};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};

pub enum DaemonCommand {
    Start(oneshot::Sender<Result<String, String>>),
    Stop(oneshot::Sender<Result<String, String>>),
    Status(oneshot::Sender<ServerState>),
    Op(oneshot::Sender<Result<(), String>>, Op),
}

pub fn spawn_actor(cfg: McConfig, mut rx: mpsc::Receiver<DaemonCommand>) {
    tokio::spawn(async move {
        println!("🤖 Actor iniciado. Listo para recibir comandos.");
        let mut srv = ServerProcess::new(cfg);

        while let Some(msg) = rx.recv().await {
            match msg {
                DaemonCommand::Start(reply) => match srv.start().await {
                    Err(e) => {
                        let _ = reply.send(Err(e.to_string()));
                    }
                    Ok(_) => {
                        let state_rx = srv.state();

                        tokio::spawn(async move {
                            let wait_result = timeout(Duration::from_secs(120), async {
                                return wait_for_state(state_rx, ServerState::Running)
                                    .await
                                    .map(|_| "Server Running");
                            })
                                .await;

                            let final_response = match wait_result {
                                Ok(Ok(msg)) => Ok(msg.to_string()),
                                Ok(Err(e)) => Err(e.to_string()),
                                Err(_) => {
                                    Err("Timeout: El servidor tardó demasiado en arrancar."
                                        .to_string())
                                }
                            };

                            let _ = reply.send(final_response);
                        });
                    }
                },
                DaemonCommand::Stop(reply) => {
                    let current_state = *srv.state().borrow();
                    if matches!(current_state, ServerState::Stopped | ServerState::Crashed) {
                        let _ = reply.send(Ok("El servidor ya estaba detenido.".to_string()));
                        continue;
                    }

                    match srv.exec_command("stop").await {
                        Err(e) => {
                            let _ = reply.send(Err(e.to_string()));
                        }
                        Ok(_) => {
                            let state_rx = srv.state();
                            tokio::spawn(async move {
                                let wait_result = timeout(Duration::from_secs(60), async {
                                    return wait_for_state(state_rx, ServerState::Stopped)
                                        .await
                                        .map(|_| "Server Stopped");
                                })
                                    .await;

                                let final_response = match wait_result {
                                    Ok(Ok(msg)) => Ok(msg.to_string()),
                                    Ok(Err(e)) => Err(e.to_string()),
                                    Err(_) => Err(
                                        "Timeout: El servidor tardó demasiado en para el servidor."
                                            .to_string(),
                                    ),
                                };

                                let _ = reply.send(final_response);
                            });
                        }
                    }
                }
                DaemonCommand::Status(reply) => {
                    let state = *srv.state().borrow();
                    let _ = reply.send(state);
                }
                DaemonCommand::Op(reply, op) => {
                    let res = srv.op(op.op, op.player.clone()).await;
                    match res {
                        Ok(()) => {
                            let _ = reply.send(Ok(()));
                        }
                        Err(_) => {
                            let _ = reply.send(Err("Error sending Command.".to_string()));
                        }
                    }
                }
            }
        }
        println!("🤖 Actor detenido (Canal cerrado).");
    });
}
