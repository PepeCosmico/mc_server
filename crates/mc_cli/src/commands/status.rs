use crate::{client::TcpClient, error::Result};
use mc_daemon::protocol::{ResponsePayload, TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = client.send_request(TcpRequest::Status).await?;

    if response.success {
        match response.data {
            ResponsePayload::Status(status) => {
                tracing::info!("Estado del servidor: {:?}", status);
            }
            _ => tracing::error!("Server returned invalid response."),
        }
    } else {
        tracing::error!("Error: {}", response.error.unwrap_or_default());
    }

    Ok(())
}
