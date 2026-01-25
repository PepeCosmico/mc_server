use crate::{client::TcpClient, error::Result};
use mc_daemon::protocol::{ResponsePayload, TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = client.send_request(TcpRequest::Start).await?;

    if response.success {
        match response.data {
            ResponsePayload::Start(start) => {
                tracing::info!("Estado del servidor: {:?}", start);
            }
            _ => tracing::error!("Server returned invalid response."),
        }
    } else {
        tracing::error!("Error: {}", response.error.unwrap_or_default());
    }

    Ok(())
}
