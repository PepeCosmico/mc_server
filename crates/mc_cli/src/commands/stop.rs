use crate::utils::run_task;
use crate::{client::TcpClient, error::Result};
use colored::Colorize;
use mc_daemon::protocol::{ResponsePayload, TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = run_task(
        "Stopping server...",
        "Server stopped",
        "Error stopping the server",
        client.send_request(TcpRequest::Stop),
    )
    .await?;

    if response.success {
        match response.data {
            ResponsePayload::Simple(msg) => {
                println!("\n");
                println!("  {}", "Minecraft Server Stop".green().bold());
                println!("  {}", "─".repeat(30).dimmed());

                println!("  {}: {}", "Server stop", msg.to_string().blue().bold());

                println!("  {}", "─".repeat(30).dimmed());
                println!();
            }
            _ => tracing::error!("Server returned invalid response."),
        }
    } else {
        tracing::error!("Error: {}", response.error.unwrap_or_default());
    }

    Ok(())
}
