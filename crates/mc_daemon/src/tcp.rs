use crate::actor::DaemonCommand;
use crate::protocol::{TcpRequest, TcpResponseBuilder};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

pub async fn server_loop(addr: &str, tx: mpsc::Sender<DaemonCommand>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("🌍 Daemon TCP escuchando en {}", addr);

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("🔌 Nueva conexión: {}", addr);
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, tx_clone).await {
                eprintln!("⚠️ Error cliente {}: {}", addr, e);
            }
        });
    }
}

async fn handle_client(
    mut socket: TcpStream,
    tx: mpsc::Sender<DaemonCommand>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = socket.split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = buf_reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

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
                writer.write_all(err_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                continue;
            }
        };

        let response_json = process_request(req, &tx).await?;

        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
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
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Op(reply_tx, op)).await.ok();
            Ok(serde_json::to_string(
                &TcpResponseBuilder::<String>::builder(true)
                    .message("Op".to_string())
                    .build(),
            )?)
        }
    }
}
