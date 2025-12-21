mod error;

use crate::error::Result;
use clap::{Parser, Subcommand};
use mcprocess::protocol::{Op, TcpRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(name = "mc")]
#[command(version = "0.1")]
#[command(about = "Cli for mc_server daemon")]
struct Cli {
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    address: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start,
    Stop,
    Status,
    Op { player_name: String, op: bool },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut stream = TcpStream::connect(cli.address).await?;

    let request = match cli.command {
        Commands::Start => TcpRequest::Start,
        Commands::Stop => TcpRequest::Stop,
        Commands::Status => TcpRequest::Status,
        Commands::Op { player_name, op } => TcpRequest::Op(Op { player_name, op }),
    };

    let json_req = serde_json::to_string(&request)?;
    stream.write_all(json_req.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let resp_str = String::from_utf8_lossy(&buf[..n]);

    let resp: serde_json::Value = serde_json::from_str(&resp_str)?;

    if resp["success"].as_bool().unwrap_or(false) {
        println!("Success: {}", resp);
        if !resp["data"].is_null() {
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }
    } else {
        eprintln!("Error: {}", resp);
    }
    Ok(())
}
