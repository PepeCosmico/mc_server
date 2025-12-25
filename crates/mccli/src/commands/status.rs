use crate::{client::TcpClient, error::Result};
use mcprocess::protocol::{TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse<String> = client.send_request(TcpRequest::Status).await?;

    if response.success {
        if let Some(msg) = response.data {
            tracing::info!("Estado del servidor: {:?}", msg);
        } else {
            tracing::warn!("Éxito, pero no llegaron datos.");
        }
    } else {
        tracing::error!("Error: {}", response.message.unwrap_or_default());
    }

    Ok(())
}
