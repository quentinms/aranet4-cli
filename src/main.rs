mod aranet;
use clap::Parser;
use serde_json::json;
use std::error::Error;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// What do you want to do
    action: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _args = Cli::parse();
    // TODO: do different actions based on args

    let central = aranet::start_scanning().await?;

    let find_aranet_res = aranet::find_aranet_peripheral(&central).await;

    if find_aranet_res.is_none() {
        return Result::Err("could not find any Aranet4 device".into());
    }

    let aranet_device = find_aranet_res.unwrap();

    let data = aranet::get_aranet_data(&aranet_device).await?;

    println!("{}", json!(data));

    Ok(())
}
