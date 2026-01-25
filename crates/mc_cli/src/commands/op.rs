use crate::{client::TcpClient, error::Result};
use mc_daemon::protocol::{Op, ResponsePayload, TcpRequest, TcpResponse};

pub async fn run(address: &str, player: String, op: bool) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = client
        .send_request(TcpRequest::Op(Op { player, op }))
        .await?;

    if response.success {
        match response.data {
            ResponsePayload::OpResult(op) => {
                tracing::info!("OP: {:?}", op);
            }
            _ => tracing::error!("Server returned invalid response."),
        }
    } else {
        tracing::error!("Error: {}", response.error.unwrap_or_default());
    }

    Ok(())
}
