use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mc")]
#[command(version = "0.1")]
#[command(about = "Cli for mc_server daemon")]
pub struct Cli {
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, default_value = "0.0.0.0:8080")]
    pub address: String,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Start,
    Stop,
    Status,
    Op {
        #[arg(long)]
        player: String,
        #[arg(long)]
        op: bool,
    },
}
