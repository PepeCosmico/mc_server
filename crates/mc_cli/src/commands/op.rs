use crate::{client::TcpClient, error::Result};
use mc_process::protocol::{Op, TcpRequest, TcpResponse};

pub async fn run(address: &str, player: String, op: bool) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse<String> = client
        .send_request(TcpRequest::Op(Op { player, op }))
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
