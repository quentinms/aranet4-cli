use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

const ARANET4_SERVICE_UUID: &str = "f0cd1400-95da-4f4b-9ac8-aa55d312af0c";
const ARANET4_CHARACTERISTIC_UUID: &str = "f0cd3001-95da-4f4b-9ac8-aa55d312af0c";

const BLUETOOTH_MODEL_NUMBER_CHARACTERISTIC: &str = "00002a24-0000-1000-8000-00805f9b34fb";
const BLUETOOTH_SERIAL_NUMBER_CHARACTERISTIC: &str = "00002a25-0000-1000-8000-00805f9b34fb";
const BLUETOOTH_FIRMWARE_REVISION_CHARACTERISTIC: &str = "00002a26-0000-1000-8000-00805f9b34fb";
const BLUETOOTH_HARDWARE_REVISION_CHARACTERISTIC: &str = "00002a27-0000-1000-8000-00805f9b34fb";
const BLUETOOTH_SOFTWARE_REVISION_CHARACTERISTIC: &str = "00002a28-0000-1000-8000-00805f9b34fb";
const BLUETOOTH_MANUFACTURER_NAME_CHARACTERISTIC: &str = "00002a29-0000-1000-8000-00805f9b34fb";

#[derive(Debug, serde::Serialize)]
pub struct Data {
    co2: u32,
    temperature: f32,
    pressure: f32,
    humidity: f32,
    battery: u32,
}

pub async fn start_scanning() -> Result<Adapter, Box<dyn std::error::Error>> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().unwrap();

    let aranet_service = Uuid::parse_str(ARANET4_SERVICE_UUID).unwrap();

    let scan_filter = ScanFilter {
        services: vec![aranet_service],
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
    let data = Data {
        co2: res[0] as u32 + (res[1] as u32) * 256,
        temperature: (res[2] as f32 + (res[3] as f32) * 256.0) / 20.0,
        pressure: (res[4] as f32 + (res[5] as f32) * 256.0) / 10.0,
        humidity: res[6] as f32,
        battery: res[7] as u32,
    };

    return Ok(data);
}

#[derive(Default, Debug, serde::Serialize)]
pub struct Info {
    model_number: Option<String>,
    serial_number: Option<String>,
    firmware_revision: Option<String>,
    hardware_revision: Option<String>,
    software_revision: Option<String>,
    manufacturer_name: Option<String>,
}

pub async fn get_info(aranet_device: &Peripheral) -> Result<Info, Box<dyn Error>> {
    aranet_device.connect().await?;

    aranet_device.discover_services().await?;

    let mut info = Info {
        ..Default::default()
    };
    for characteristic in aranet_device.characteristics() {
        match characteristic.uuid.to_string().as_str() {
            BLUETOOTH_MODEL_NUMBER_CHARACTERISTIC => {
                let res = aranet_device.read(&characteristic).await?;
                info.model_number = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_SERIAL_NUMBER_CHARACTERISTIC => {
                let res = aranet_device.read(&characteristic).await?;
                info.serial_number = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_FIRMWARE_REVISION_CHARACTERISTIC => {
                let res = aranet_device.read(&characteristic).await?;
                info.firmware_revision = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_HARDWARE_REVISION_CHARACTERISTIC => {
                let res = aranet_device.read(&characteristic).await?;
                info.hardware_revision = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_SOFTWARE_REVISION_CHARACTERISTIC => {
                let res = aranet_device.read(&characteristic).await?;
                info.software_revision = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_MANUFACTURER_NAME_CHARACTERISTIC => {
                let res = aranet_device.read(&characteristic).await?;
                info.manufacturer_name = Some(String::from_utf8_lossy(&res).to_string());
            }
            _ => {}
        }
    }

    return Ok(info);
}
