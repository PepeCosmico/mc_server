use crate::utils::run_task;
use crate::{client::TcpClient, error::Result};
use colored::Colorize;
use mc_types::tcp::protocol::{ResponsePayload, TcpRequest, TcpResponse};
use mc_types::tcp::schemas::Op;

pub async fn run(address: &str, player: String, op: bool) -> Result<()> {
    let client = TcpClient::new(address);

    let response: TcpResponse = run_task(
        "Give / Remove op permit...",
        "Gave / Removed op permit",
        "Error with op permit",
        client.send_request(TcpRequest::Op(Op { player, op })),
    )
    .await?;

    if response.success {
        match response.data {
            ResponsePayload::OpResult(op) => {
                println!("\n");
                println!("  {}", "Minecraft Server Status".green().bold());
                println!("  {}", "─".repeat(30).dimmed());

                println!("  Player: {}", op.player);
                println!("  Action: {}", if op.op { "Op" } else { "Deop" });

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
