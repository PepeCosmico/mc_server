use crate::utils::wait_for_state;
use mc_config::McConfig;
use mc_process::server::ServerProcess;
use mc_types::server::{event::ServerEvent, state::ServerState};
use mc_types::tcp::schemas::Op;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

pub enum DaemonCommand {
    Start(oneshot::Sender<Result<(String, String), String>>),
    Stop(oneshot::Sender<Result<String, String>>),
    Status(oneshot::Sender<ServerState>),
    Op(oneshot::Sender<Result<(), String>>, Op),
    Shutdown(oneshot::Sender<Result<(), String>>),
}

pub fn spawn_actor(cfg: McConfig, mut rx: mpsc::Receiver<DaemonCommand>) {
    tokio::spawn(async move {
        info!("actor started, ready for commands");
        let mut srv = ServerProcess::new(cfg.clone());

        spawn_state_watcher(srv.state());
        spawn_event_watcher(srv.logs());

        while let Some(msg) = rx.recv().await {
            match msg {
                DaemonCommand::Start(reply) => start(&mut srv, reply, &cfg).await,
                DaemonCommand::Stop(reply) => stop(&mut srv, reply, &cfg).await,
                DaemonCommand::Status(reply) => status(&srv, reply),
                DaemonCommand::Op(reply, op_data) => op(&mut srv, reply, op_data).await,
                DaemonCommand::Shutdown(reply) => {
                    shutdown(&mut srv, reply, &cfg).await;
                    break;
                }
            }
        }
        info!("actor stopped (channel closed)");
    });
}

async fn start(
    srv: &mut ServerProcess,
    reply: oneshot::Sender<Result<(String, String), String>>,
    cfg: &McConfig,
) {
    match srv.start().await {
        Err(e) => {
            let _ = reply.send(Err(e.to_string()));
        }
        Ok(_) => {
            let state_rx = srv.state();
            let version_rx = srv.version();
            let address = cfg.client.get_addr();
            let kill = srv.kill_handle();
            let start_timeout = cfg.daemon.start_timeout_secs;
            let force_timeout = cfg.daemon.force_kill_timeout_secs;

            tokio::spawn(async move {
                let wait_result = timeout(
                    Duration::from_secs(start_timeout),
                    wait_for_state(state_rx.clone(), ServerState::Running),
                )
                .await;

                let version = match version_rx.borrow().clone() {
                    Some(ver) => ver.mc_version,
                    None => "Unknown".to_string(),
                };

                let final_response = match wait_result {
                    Ok(Ok(())) => Ok((version, address)),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(_) => {
                        warn!("start timed out after {start_timeout}s, escalating to force-kill");
                        kill.notify_one();
                        let _ = timeout(
                            Duration::from_secs(force_timeout),
                            wait_for_state(state_rx, ServerState::Stopped),
                        )
                        .await;
                        Err(format!("Timeout: el servidor tardó más de {start_timeout}s en arrancar."))
                    }
                };

                let _ = reply.send(final_response);
            });
        }
    }
}

async fn stop(
    srv: &mut ServerProcess,
    reply: oneshot::Sender<Result<String, String>>,
    cfg: &McConfig,
) {
    let current_state = *srv.state().borrow();
    if matches!(current_state, ServerState::Stopped | ServerState::Crashed) {
        let _ = reply.send(Ok("El servidor ya estaba detenido.".to_string()));
        return;
    }

    if matches!(current_state, ServerState::Running | ServerState::Starting) {
        if let Err(e) = srv.stop().await {
            let _ = reply.send(Err(e.to_string()));
            return;
        }
    }

    let state_rx = srv.state();
    let kill = srv.kill_handle();
    let stop_timeout = cfg.daemon.stop_timeout_secs;
    let force_timeout = cfg.daemon.force_kill_timeout_secs;

    tokio::spawn(async move {
        let graceful = timeout(
            Duration::from_secs(stop_timeout),
            wait_for_state(state_rx.clone(), ServerState::Stopped),
        )
        .await;

        if let Ok(Ok(())) = graceful {
            let _ = reply.send(Ok("Server Stopped".to_string()));
            return;
        }

        warn!("stop timed out after {stop_timeout}s, escalating to force-kill");
        kill.notify_one();

        let forced = timeout(
            Duration::from_secs(force_timeout),
            wait_for_state(state_rx, ServerState::Stopped),
        )
        .await;

        let response = match forced {
            Ok(Ok(())) => Ok("Server force-killed".to_string()),
            _ => Err("force kill did not stop the jvm".to_string()),
        };
        let _ = reply.send(response);
    });
}

fn status(srv: &ServerProcess, reply: oneshot::Sender<ServerState>) {
    let status = srv.state().borrow().clone();
    let _ = reply.send(status);
}

/// Graceful shutdown: send `/stop`, wait for `Stopped`, escalate to force-kill
/// on timeout. Runs inline (not in a spawned task) because no further actor
/// commands will be processed after this completes — the daemon is exiting.
async fn shutdown(
    srv: &mut ServerProcess,
    reply: oneshot::Sender<Result<(), String>>,
    cfg: &McConfig,
) {
    let current = *srv.state().borrow();
    if matches!(current, ServerState::Stopped | ServerState::Crashed) {
        let _ = reply.send(Ok(()));
        return;
    }

    if matches!(current, ServerState::Running | ServerState::Starting) {
        let _ = srv.stop().await;
    }

    let graceful = timeout(
        Duration::from_secs(cfg.daemon.stop_timeout_secs),
        wait_for_state(srv.state(), ServerState::Stopped),
    )
    .await;

    if matches!(graceful, Ok(Ok(()))) {
        let _ = reply.send(Ok(()));
        return;
    }

    warn!("graceful shutdown timed out, forcing kill");
    srv.force_stop();

    let forced = timeout(
        Duration::from_secs(cfg.daemon.force_kill_timeout_secs),
        wait_for_state(srv.state(), ServerState::Stopped),
    )
    .await;

    match forced {
        Ok(Ok(())) => {
            let _ = reply.send(Ok(()));
        }
        _ => {
            let _ = reply.send(Err("force kill did not stop the jvm".to_string()));
        }
    }
}

fn spawn_state_watcher(mut state_rx: tokio::sync::watch::Receiver<ServerState>) {
    tokio::spawn(async move {
        let mut prev = *state_rx.borrow();
        info!("state: {prev:?}");
        while state_rx.changed().await.is_ok() {
            let next = *state_rx.borrow();
            info!("state: {prev:?} -> {next:?}");
            prev = next;
        }
    });
}

fn spawn_event_watcher(mut log_rx: tokio::sync::broadcast::Receiver<mc_process::logs::McLog>) {
    tokio::spawn(async move {
        loop {
            match log_rx.recv().await {
                Ok(log) => match log.event {
                    ServerEvent::Unknown => {}
                    ServerEvent::Chat { .. } => debug!("event: {:?}", log.event),
                    other => info!("event: {other:?}"),
                },
                Err(RecvError::Lagged(n)) => warn!("event channel lagged, dropped {n} entries"),
                Err(RecvError::Closed) => break,
            }
        }
    });
}

async fn op(srv: &mut ServerProcess, reply: oneshot::Sender<Result<(), String>>, op: Op) {
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
