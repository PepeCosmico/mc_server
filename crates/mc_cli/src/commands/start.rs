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
        match response.data {
            ResponsePayload::Start(start) => {
                tracing::info!("Estado del servidor: {:?}", start);
            }
            _ => tracing::error!("Server returned invalid response."),
        }
    } else {
        tracing::error!("Error: {}", response.message.unwrap_or_default());
    }

    Ok(())
}
