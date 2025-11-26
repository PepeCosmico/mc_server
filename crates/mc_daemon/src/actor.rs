use mcprocess::config::Config;
use mcprocess::server::{ServerProcess, ServerState};
use tokio::sync::{mpsc, oneshot};

// Comandos internos (hilos -> actor)
pub enum DaemonCommand {
    Start(oneshot::Sender<Result<String, String>>),
    Stop(oneshot::Sender<Result<String, String>>),
    Kill(oneshot::Sender<()>),
    Status(oneshot::Sender<ServerState>),
    Input {
        cmd: String,
        resp: oneshot::Sender<Result<String, String>>,
    },
    Backup(oneshot::Sender<Result<String, String>>),
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
                    let res = srv
                        .start()
                        .await
                        .map(|_| "Server started".to_string())
                        .map_err(|e| e.to_string());
                    let _ = reply.send(res);
                }
                DaemonCommand::Stop(reply) => {
                    let res = srv
                        .stop(30)
                        .await
                        .map(|_| "Server stopped".to_string())
                        .map_err(|e| e.to_string());
                    let _ = reply.send(res);
                }
                DaemonCommand::Kill(reply) => {
                    //TODO
                }
                DaemonCommand::Status(reply) => {
                    let state = *srv.state().borrow();
                    let _ = reply.send(state);
                }
                DaemonCommand::Input { cmd, resp: reply } => {
                    let res = srv
                        .exec_command(&cmd)
                        .await
                        .map(|_| format!("Command sent: {}", cmd))
                        .map_err(|e| e.to_string());
                    let _ = reply.send(res);
                }
                DaemonCommand::Backup(reply) => {
                    let res = srv
                        .backup()
                        .await
                        .map(|f_name| {
                            serde_json::json!({"success": true, "file": f_name}).to_string()
                        })
                        .map_err(|e| e.to_string());
                    let _ = reply.send(res);
                }
            }
        }
        println!("🤖 Actor detenido (Canal cerrado).");
    });
}
