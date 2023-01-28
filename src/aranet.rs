use btleplug::api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::stream::StreamExt;
use std::error::Error;
use std::time;
use uuid::{uuid, Uuid};

const ARANET4_SERVICE: Uuid = uuid!("0000fce0-0000-1000-8000-00805f9b34fb");

const ARANET4_CHARACTERISTIC: Uuid = uuid!("f0cd3001-95da-4f4b-9ac8-aa55d312af0c");

const BLUETOOTH_MODEL_NUMBER_CHARACTERISTIC: Uuid = uuid!("00002a24-0000-1000-8000-00805f9b34fb");
const BLUETOOTH_SERIAL_NUMBER_CHARACTERISTIC: Uuid = uuid!("00002a25-0000-1000-8000-00805f9b34fb");
const BLUETOOTH_FIRMWARE_REVISION_CHARACTERISTIC: Uuid =
    uuid!("00002a26-0000-1000-8000-00805f9b34fb");
const BLUETOOTH_HARDWARE_REVISION_CHARACTERISTIC: Uuid =
    uuid!("00002a27-0000-1000-8000-00805f9b34fb");
const BLUETOOTH_SOFTWARE_REVISION_CHARACTERISTIC: Uuid =
    uuid!("00002a28-0000-1000-8000-00805f9b34fb");
const BLUETOOTH_MANUFACTURER_NAME_CHARACTERISTIC: Uuid =
    uuid!("00002a29-0000-1000-8000-00805f9b34fb");

#[derive(Default, Debug, serde::Serialize)]
pub struct Device {
    name: String,
    address: BDAddr,
    data: Data,
    info: Info,
}

#[derive(Default, Debug, serde::Serialize)]
pub struct Data {
    co2: u32,
    temperature: f32,
    pressure: f32,
    humidity: f32,
    battery: u32,
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

pub async fn get_devices(
    max_devices: Option<usize>,
    timeout: time::Duration,
) -> Result<Vec<Device>, Box<dyn std::error::Error>> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().next().unwrap();

    let scan_filter = ScanFilter {
        services: vec![ARANET4_SERVICE],
    };
    // start scanning for devices
    central.start_scan(scan_filter).await?;

    // Based on https://github.com/deviceplug/btleplug/blob/21947d6f6e23466b6d06e523b1ffa48bb5a227b3/examples/event_driven_discovery.rs
    let mut events = central.events().await?;

    let start = time::Instant::now();
    let mut devices: Vec<Device> = Vec::new();

    loop {
        // I don't really like this but I could't figure out a better way to exit early
        match events.next().await {
            Some(CentralEvent::DeviceDiscovered(id))
                if time::Instant::now().duration_since(start) <= time::Duration::from_secs(10)
                    && max_devices.map(|max| devices.len() < max).unwrap_or(true) =>
            {
                let aranet_device = central.peripheral(&id).await.unwrap();

                let device = get_device(&aranet_device).await?;

                devices.push(device);
            }
            Some(_)
                if time::Instant::now().duration_since(start) <= timeout
                    && max_devices.map(|max| devices.len() < max).unwrap_or(true) =>
            {
                // Do nothing for other events while we can still look for devices
            }
            _ => {
                break;
            }
        }
    }

    return Ok(devices);
}

async fn get_device(aranet_device: &Peripheral) -> Result<Device, Box<dyn Error>> {
    aranet_device.connect().await?;

    // discover services and characteristics
    aranet_device.discover_services().await?;

    let mut device = Device {
        ..Default::default()
    };

    device.name = get_name(aranet_device).await;

    eprintln!("Found {:?}", device.name);

    device.data = get_data(aranet_device).await?;
    device.info = get_info(aranet_device).await?;

    Ok(device)
}

async fn get_name(device: &Peripheral) -> String {
    let properties = device.properties().await.unwrap().unwrap();
    let name = properties.local_name.unwrap();
    return name;
}

async fn get_data(device: &Peripheral) -> Result<Data, Box<dyn Error>> {
    let chars = device.characteristics();
    let data_char = chars
        .iter()
        .find(|c| c.uuid == ARANET4_CHARACTERISTIC)
        .unwrap();

    let res = device.read(data_char).await?;

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

async fn get_info(device: &Peripheral) -> Result<Info, Box<dyn Error>> {
    let mut info = Info {
        ..Default::default()
    };
    for characteristic in device.characteristics() {
        match characteristic.uuid {
            BLUETOOTH_MODEL_NUMBER_CHARACTERISTIC => {
                let res = device.read(&characteristic).await?;
                info.model_number = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_SERIAL_NUMBER_CHARACTERISTIC => {
                let res = device.read(&characteristic).await?;
                info.serial_number = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_FIRMWARE_REVISION_CHARACTERISTIC => {
                let res = device.read(&characteristic).await?;
                info.firmware_revision = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_HARDWARE_REVISION_CHARACTERISTIC => {
                let res = device.read(&characteristic).await?;
                info.hardware_revision = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_SOFTWARE_REVISION_CHARACTERISTIC => {
                let res = device.read(&characteristic).await?;
                info.software_revision = Some(String::from_utf8_lossy(&res).to_string());
            }
            BLUETOOTH_MANUFACTURER_NAME_CHARACTERISTIC => {
                let res = device.read(&characteristic).await?;
                info.manufacturer_name = Some(String::from_utf8_lossy(&res).to_string());
            }
            _ => {}
        }
    }

    return Ok(info);
}
