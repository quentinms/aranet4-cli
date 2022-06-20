use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use clap::Parser;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct Cli {
    /// What do you want to do
    action: String,
}

const ARANET4_SERVICE_UUID: &str = "f0cd1400-95da-4f4b-9ac8-aa55d312af0c";
const ARANET4_CHARACTERISTIC_UUID: &str = "f0cd3001-95da-4f4b-9ac8-aa55d312af0c";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _args = Cli::parse();
    // TODO: do different actions based on args

    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().unwrap();

    let service_uuid = Uuid::parse_str(ARANET4_SERVICE_UUID).unwrap();

    // let scan_filter = ScanFilter::default();
    let scan_filter = ScanFilter {
        services: vec![service_uuid],
    };
    // start scanning for devices
    central.start_scan(scan_filter).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    time::sleep(Duration::from_secs(10)).await;

    // find the device we're interested in
    let find_aranet_res = find_aranet_peripheral(&central).await;

    if find_aranet_res.is_none() {
        return Result::Err("Could not find Aranet4 device".into());
    }

    let aranet_device = find_aranet_res.unwrap();

    // connect to the device
    aranet_device.connect().await?;

    // discover services and characteristics
    aranet_device.discover_services().await?;

    let aranet4_characteristic: Uuid = Uuid::parse_str(ARANET4_CHARACTERISTIC_UUID).unwrap();

    // find the characteristic we want
    let chars = aranet_device.characteristics();
    let data_char = chars
        .iter()
        .find(|c| c.uuid == aranet4_characteristic)
        .unwrap();

    let res = aranet_device.read(data_char).await?;

    println!("CO2: {:?}", u32::from(res[0]) + u32::from(res[1]) * 256);
    println!(
        "Temp: {:?}",
        (u32::from(res[2]) + u32::from(res[3]) * 256) / 20
    );
    println!(
        "Pressure: {:?}",
        (u32::from(res[4]) + u32::from(res[5]) * 256) / 10
    );
    println!("Humidity: {:?}", u32::from(res[6]));
    println!("Battery: {:?}", u32::from(res[7]));

    Ok(())
}

async fn find_aranet_peripheral(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        let properties = p.properties().await.unwrap().unwrap();
        let name = properties.local_name.unwrap();
        println!("Found {:?}", name);
        if name.contains("Aranet") {
            return Some(p);
        }
    }
    None
}
