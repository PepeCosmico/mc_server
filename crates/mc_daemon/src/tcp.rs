use crate::actor::DaemonCommand;
use crate::protocol::{ResponsePayload, StartData, TcpRequest, TcpResponse};
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_util::codec::{Framed, LinesCodec};

pub async fn server_loop(addr: &str, tx: mpsc::Sender<DaemonCommand>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Daemon TCP escuchando en {}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((socket, addr)) => {
                        println!("Nueva conexión: {}", addr);
                        let tx_clone = tx.clone();

                        tokio::spawn(async move {
                            if let Err(e) = handle_client(socket, tx_clone).await {
                                eprintln!("Error cliente {}: {}", addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        println!("Error accepting connection: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Stopping Daemon...");
                break;
            }
        };
    }
    Ok(())
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
            let (reply_tx, _reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Op(reply_tx, op.clone())).await.ok();
            Ok(serde_json::to_string(&TcpResponse::ok_with(
                ResponsePayload::OpResult(op),
            ))?)
        }
    }
}
