mod aranet;
use btleplug::platform::Peripheral;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::error::Error;

/// Get data from your Aranet4 device.
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Get status of the aranet device
    Status {},
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let central = aranet::start_scanning().await?;

    let find_aranet_res = aranet::find_aranet_peripheral(&central).await;

    if find_aranet_res.is_none() {
        return Result::Err("could not find any Aranet4 device".into());
    }

    let aranet_device = find_aranet_res.unwrap();

    match &cli.command {
        Some(Commands::Status {}) => {
            get_status(aranet_device).await?;
        }
        None => {}
    }

    Ok(())
}

async fn get_status(aranet_device: Peripheral) -> Result<(), Box<dyn Error>> {
    let data = aranet::get_aranet_data(&aranet_device).await?;

    println!("{}", json!(data));
    Ok(())
}
