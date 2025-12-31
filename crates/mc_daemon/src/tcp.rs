use crate::actor::DaemonCommand;
use crate::protocol::{TcpRequest, TcpResponseBuilder};
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
                        let err_json = serde_json::to_string(
                            &TcpResponseBuilder::<()>::builder(false)
                                .message(format!("JSON inválido: {}", e))
                                .build(),
                        )?;
                        framed.send(err_json).await?;
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
            Ok(serde_json::to_string(
                &TcpResponseBuilder::builder(true).data(state).build(),
            )?)
        }
        TcpRequest::Start => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Start(reply_tx)).await.ok();
            match reply_rx.await? {
                Ok(msg) => Ok(serde_json::to_string(
                    &TcpResponseBuilder::<()>::builder(true).message(msg).build(),
                )?),
                Err(e) => Ok(serde_json::to_string(
                    &TcpResponseBuilder::<()>::builder(false).message(e).build(),
                )?),
            }
        }
        TcpRequest::Stop => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Stop(reply_tx)).await.ok();
            match reply_rx.await? {
                Ok(msg) => Ok(serde_json::to_string(
                    &TcpResponseBuilder::<()>::builder(true).message(msg).build(),
                )?),
                Err(e) => Ok(serde_json::to_string(
                    &TcpResponseBuilder::<String>::builder(false)
                        .message(e)
                        .build(),
                )?),
            }
        }
        TcpRequest::Op(op) => {
            let (reply_tx, _reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Op(reply_tx, op.clone())).await.ok();
            Ok(serde_json::to_string(
                &TcpResponseBuilder::<String>::builder(true)
                    .message(if op.op {
                        "Op".to_string()
                    } else {
                        "Deop".to_string()
                    })
                    .build(),
            )?)
        }
    }
}
