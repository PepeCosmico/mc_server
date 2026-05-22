use crate::actor::DaemonCommand;
use futures::{SinkExt, StreamExt};
use mc_types::tcp::protocol::{ResponsePayload, TcpRequest, TcpResponse};
use mc_types::tcp::schemas::StartData;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{error, info};

pub async fn server_loop(addr: &str, tx: mpsc::Sender<DaemonCommand>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    #[cfg(windows)]
    deny_socket_inheritance(&listener)?;
    info!("daemon TCP listening on {}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((socket, addr)) => {
                        let tx_clone = tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_client(socket, tx_clone).await {
                                error!("client {} error: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("error accepting connection: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("stopping daemon...");
                graceful_shutdown(&tx).await;
                break;
            }
        }
    }
    Ok(())
}

/// Explicitly clear `HANDLE_FLAG_INHERIT` on the listener socket. Belt and
/// braces against child processes (the spawned JVM) inheriting the bind and
/// keeping port 7110 stuck after the daemon dies abruptly.
#[cfg(windows)]
fn deny_socket_inheritance(listener: &TcpListener) -> std::io::Result<()> {
    use std::os::windows::io::AsRawSocket;
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT};

    let handle = listener.as_raw_socket() as HANDLE;
    let ok = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Coordinates the actor's `Shutdown` reply with an outer safety timeout, so
/// even if the actor itself hangs we still return and let `main` exit.
async fn graceful_shutdown(tx: &mpsc::Sender<DaemonCommand>) {
    let (reply_tx, reply_rx) = oneshot::channel();
    if tx.send(DaemonCommand::Shutdown(reply_tx)).await.is_err() {
        error!("actor channel closed");
        return;
    }
    // Outer deadline larger than the actor's internal 30s graceful + 5s force,
    // in case the actor is still draining a previous command.
    match tokio::time::timeout(Duration::from_secs(45), reply_rx).await {
        Ok(Ok(Ok(()))) => info!("daemon stopped"),
        Ok(Ok(Err(e))) => error!("shutdown error: {e}"),
        Ok(Err(_)) => error!("actor exited without reply"),
        Err(_) => error!("actor reply timed out"),
    }
}

async fn handle_client(socket: TcpStream, tx: mpsc::Sender<DaemonCommand>) -> anyhow::Result<()> {
    let mut framed = Framed::new(socket, LinesCodec::new());

    while let Some(result) = framed.next().await {
        match result {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let req: TcpRequest = match serde_json::from_str(trimmed) {
                    Ok(val) => val,
                    Err(e) => {
                        let err_res = serde_json::to_string(&TcpResponse::error(&format!(
                            "Invalid Json: {}",
                            e
                        )))?;
                        framed.send(err_res).await?;
                        continue;
                    }
                };

                let response_json = process_request(req, &tx).await?;

                framed.send(response_json).await?;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    Ok(())
}

async fn process_request(
    req: TcpRequest,
    tx: &mpsc::Sender<DaemonCommand>,
) -> anyhow::Result<String> {
    info!("incoming request: {req:?}");
    match req {
        TcpRequest::Status => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Status(reply_tx)).await.ok();
            let state = reply_rx.await?;
            Ok(serde_json::to_string(&TcpResponse::ok_with(
                ResponsePayload::Status(state),
            ))?)
        }
        TcpRequest::Start => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Start(reply_tx)).await.ok();
            match reply_rx.await? {
                Ok((version, address)) => Ok(serde_json::to_string(&TcpResponse::ok_with(
                    ResponsePayload::Start(StartData { version, address }),
                ))?),
                Err(e) => Ok(serde_json::to_string(&TcpResponse::error(&e))?),
            }
        }
        TcpRequest::Stop => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Stop(reply_tx)).await.ok();
            match reply_rx.await? {
                Ok(msg) => Ok(serde_json::to_string(&TcpResponse::ok_with(
                    ResponsePayload::Simple(msg),
                ))?),
                Err(e) => Ok(serde_json::to_string(&TcpResponse::error(&e))?),
            }
        }
        TcpRequest::Op(op) => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Op(reply_tx, op.clone())).await.ok();
            match reply_rx.await? {
                Ok(()) => Ok(serde_json::to_string(&TcpResponse::ok_with(
                    ResponsePayload::OpResult(op),
                ))?),
                Err(e) => Ok(serde_json::to_string(&TcpResponse::error(&e))?),
            }
        }
    }
}
