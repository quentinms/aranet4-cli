mod aranet;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::error::Error;
use std::time;

/// Get data from your Aranet4 device.
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Get aranet devices and their data
    Get {
        /// How long to wait for devices to be detected, in seconds
        #[clap(short, long, default_value = "10")]
        timeout: u64,
        /// How many devices to look for
        #[clap(short, long)]
        max_devices: Option<usize>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Get {
            timeout,
            max_devices,
        }) => {
            let devices =
                aranet::get_devices(*max_devices, time::Duration::from_secs(*timeout)).await?;
            println!("{}", json!(devices));
        }
        None => {}
    }

    Ok(())
}
