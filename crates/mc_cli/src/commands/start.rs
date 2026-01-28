use crate::{client::TcpClient, error::Result, utils::run_task};
use colored::Colorize;
use mc_daemon::protocol::{ResponsePayload, TcpRequest, TcpResponse};

pub async fn run(address: &str) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = run_task(
        "Starting server...",
        "Server started successfully",
        "Error starting server",
        client.send_request(TcpRequest::Start),
    )
    .await?;

    if response.success {
        match response.data {
            ResponsePayload::Start(start) => {
                println!("\n");
                println!("  {}", "Minecraft Server Online".green().bold());
                println!("  {}", "─".repeat(30).dimmed());

                println!("  {}: {}", "Version".bold(), start.version.cyan());

                let full_addr = format!("{}", start.address);
                println!("  {}: {}", "Address".bold(), full_addr.yellow().bold());

                println!("  {}", "─".repeat(30).dimmed());
                println!("  {}", "You can connect now.".italic().dimmed());
            }
            _ => tracing::error!("Server returned invalid response."),
        }
    } else {
        tracing::error!("Error: {:?}", response);
    }

    Ok(())
}
