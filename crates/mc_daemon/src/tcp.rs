use crate::actor::DaemonCommand;
use crate::protocol::{TcpRequest, TcpResponse, TcpResponseBuilder};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

/// Bucle principal que acepta conexiones
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

/// Lógica por cliente
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
        } // EOF

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 1. Parsear
        let req: TcpRequest = match serde_json::from_str(trimmed) {
            Ok(val) => val,
            Err(e) => {
                let err_json = serde_json::to_string(&TcpResponse::<()>::error(format!(
                    "JSON inválido: {}",
                    e
                )))?;
                writer.write_all(err_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                continue;
            }
        };

        // 2. Procesar (Comunicación con Actor)
        let response_json = process_request(req, &tx).await?;

        // 3. Responder
        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }
    Ok(())
}

/// Helper para traducir Request -> Actor Command -> Response JSON
async fn process_request(
    req: TcpRequest,
    tx: &mpsc::Sender<DaemonCommand>,
) -> anyhow::Result<String> {
    match req {
        TcpRequest::Status => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Status(reply_tx)).await.ok();
            let state = reply_rx.await?;
            Ok(serde_json::to_string(&TcpResponse::data(state))?)
        }
        TcpRequest::Start => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Start(reply_tx)).await.ok();
            match reply_rx.await? {
                Ok(msg) => Ok(serde_json::to_string(
                    &TcpResponseBuilder::<()>::builder(true).message(msg).build(),
                )?),
                Err(e) => Ok(serde_json::to_string(&TcpResponse::<()>::error(e))?),
            }
        }
        TcpRequest::Stop => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Stop(reply_tx)).await.ok();
            wait_reply(reply_rx).await
        }
        TcpRequest::Input(cmd_str) => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Input {
                cmd: cmd_str,
                resp: reply_tx,
            })
            .await
            .ok();
            wait_reply(reply_rx).await
        }
        TcpRequest::Backup => {
            let (reply_tx, reply_rx) = oneshot::channel();
            tx.send(DaemonCommand::Backup(reply_tx)).await.ok();
            match reply_rx.await? {
                Ok(fname) => Ok(serde_json::json!({ "success": true, "data": fname }).to_string()),
                Err(e) => Ok(serde_json::to_string(&TcpResponse::<()>::error(e))?),
            }
        }
        _ => Ok(serde_json::json!({ "success": false, "message": "No implementado" }).to_string()),
    }
}

async fn wait_reply(reply_rx: oneshot::Receiver<Result<String, String>>) -> anyhow::Result<String> {
    match reply_rx.await? {
        Ok(msg) => Ok(serde_json::to_string(&TcpResponse::<()>::success(msg))?),
        Err(e) => Ok(serde_json::to_string(&TcpResponse::<()>::error(e))?),
    }
}
