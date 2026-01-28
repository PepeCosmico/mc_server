use crate::utils::run_task;
use crate::{client::TcpClient, error::Result};
use colored::Colorize;
use mc_daemon::protocol::{ResponsePayload, TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = run_task(
        "Getting status...",
        "Status received",
        "Error getting status",
        client.send_request(TcpRequest::Status),
    )
    .await?;

    if response.success {
        match response.data {
            ResponsePayload::Status(status) => {
                println!("\n");
                println!("  {}", "Minecraft Server Status".green().bold());
                println!("  {}", "─".repeat(30).dimmed());

                println!(
                    "  {}: {}",
                    "Server status",
                    status.to_string().blue().bold()
                );

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
