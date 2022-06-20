use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

const ARANET4_SERVICE_UUID: &str = "f0cd1400-95da-4f4b-9ac8-aa55d312af0c";
const ARANET4_CHARACTERISTIC_UUID: &str = "f0cd3001-95da-4f4b-9ac8-aa55d312af0c";

#[derive(Debug, serde::Serialize)]
pub struct Data {
    co2: u32,
    temperature: f32,
    pressure: f32,
    humidity: f32,
    battery: u32,
}

impl Data {
    pub fn new(co2: u32, temperature: f32, pressure: f32, humidity: f32, battery: u32) -> Self {
        Self {
            co2,
            temperature,
            pressure,
            humidity,
            battery,
        }
    }
}

pub async fn start_scanning() -> Result<Adapter, Box<dyn std::error::Error>> {
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
    time::sleep(Duration::from_secs(10)).await; // TODO: config
    return Ok(central);
}

pub async fn find_aranet_peripheral(central: &Adapter) -> Option<Peripheral> {
    // TODO: handle multiple devices
    for p in central.peripherals().await.unwrap() {
        let properties = p.properties().await.unwrap().unwrap();
        let name = properties.local_name.unwrap();
        eprintln!("Found {:?}", name);
        if name.contains("Aranet") {
            return Some(p);
        }
    }
    None
}

pub async fn get_aranet_data(aranet_device: &Peripheral) -> Result<Data, Box<dyn Error>> {
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

    // Adapted from https://github.com/SAF-Tehnika-Developer/com.aranet4/blob/54ec587f49cdece2236528edf0b871c259eb220c/app.js#L175-L182
    let data = Data::new(
        res[0] as u32 + (res[1] as u32) * 256,            // CO2
        (res[2] as f32 + (res[3] as f32) * 256.0) / 20.0, // temperature
        (res[4] as f32 + (res[5] as f32) * 256.0) / 10.0, // pressure
        res[6] as f32,                                    // humidity
        res[7] as u32,                                    // battery
    );

    return Ok(data);
}
