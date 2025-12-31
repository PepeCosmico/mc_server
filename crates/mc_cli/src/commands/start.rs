use crate::{client::TcpClient, error::Result, utils::run_task};
use mc_daemon::protocol::{TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse<String> = run_task(
        "Starting server...",
        "Server started successfully",
        "Error starting server",
        client.send_request(TcpRequest::Start),
    )
    .await?;

    if response.success {
        if let Some(msg) = response.message {
            tracing::info!("{}", msg);
        }
    } else {
        tracing::error!("Error: {}", response.message.unwrap_or_default());
    }

    Ok(())
}
