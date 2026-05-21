use crate::error::Result;
use futures::{SinkExt, StreamExt};
use mc_types::tcp::protocol::TcpResponse;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{debug, error, info, warn};

pub struct TcpClient {
    address: String,
}

impl TcpClient {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub async fn send_request<R>(&self, request: R) -> Result<TcpResponse>
    where
        R: Serialize + std::fmt::Debug,
    {
        debug!("Attempting to establish TCP connection");
        let stream = TcpStream::connect(&self.address).await?;
        info!("Connection established successfully");
        let mut framed = Framed::new(stream, LinesCodec::new());

        debug!(payload = ?request, "Sending request payload");
        let req_json = serde_json::to_string(&request)?;

        framed.send(req_json).await?;
        debug!("Waiting for response...");

        if let Some(result) = framed.next().await {
            match result {
                Ok(line) => {
                    let response: TcpResponse = serde_json::from_str(&line)?;
                    if !response.success {
                        warn!(
                            msg = ?response.error,
                            "Server processed request but returned failure status"
                        );
                    } else {
                        info!("Request processed successfully")
                    }
                    Ok(response)
                }
                Err(e) => {
                    error!(error = %e, "Network error while reading frame");
                    panic!("Error reading response: {}", e);
                }
            }
        } else {
            warn!("Connection closed by remote peer unexpectedly");
            panic!("Server ended connection wthout responding");
        }
    }
}
