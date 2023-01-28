use btleplug::api::{BDAddr, Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::stream::StreamExt;
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
) -> anyhow::Result<Vec<Device>> {
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

    let mut devices = Vec::new();
    if !max_devices.map(|m| devices.len() < m).unwrap_or(true) {
        // In case we are requested to find 0 devices, return early.
        return Ok(devices);
    }

    let timeout_instant = time::Instant::now() + timeout;
    while let Ok(Some(event)) = tokio::time::timeout_at(timeout_instant.into(), events.next()).await
    {
        if let CentralEvent::DeviceDiscovered(id) = event {
            let device = central.peripheral(&id).await.unwrap();
            let services = get_services(&device).await?;
            // The ScanFilter is only best effort and some implementation might return devices that
            // do not offer the requested service.
            if !services.contains(&ARANET4_SERVICE) {
                continue;
            }
            devices.push(get_device(&device).await?);

            if !max_devices.map(|m| devices.len() < m).unwrap_or(true) {
                return Ok(devices);
            }
        }
    }
    central.stop_scan().await?;

    Ok(devices)
}

async fn get_device(aranet_device: &Peripheral) -> anyhow::Result<Device> {
    aranet_device.connect().await?;

    // discover services and characteristics
    aranet_device.discover_services().await?;

    Ok(Device {
        name: get_name(aranet_device).await?,
        address: aranet_device.address(),
        data: get_data(aranet_device).await?,
        info: get_info(aranet_device).await?,
    })
}

async fn get_name(device: &Peripheral) -> anyhow::Result<String> {
    let properties = device.properties().await?.unwrap();
    Ok(properties.local_name.unwrap())
}

async fn get_services(device: &Peripheral) -> anyhow::Result<Vec<Uuid>> {
    let properties = device.properties().await?.unwrap();
    Ok(properties.services)
}

async fn get_data(device: &Peripheral) -> anyhow::Result<Data> {
    let chars = device.characteristics();
    let data_char = chars
        .iter()
        .find(|c| c.uuid == ARANET4_CHARACTERISTIC)
        .unwrap();

    let res = device.read(data_char).await?;

    // Adapted from https://github.com/SAF-Tehnika-Developer/com.aranet4/blob/54ec587f49cdece2236528edf0b871c259eb220c/app.js#L175-L182
    Ok(Data {
        co2: res[0] as u32 + (res[1] as u32) * 256,
        temperature: (res[2] as f32 + (res[3] as f32) * 256.0) / 20.0,
        pressure: (res[4] as f32 + (res[5] as f32) * 256.0) / 10.0,
        humidity: res[6] as f32,
        battery: res[7] as u32,
    })
}

async fn get_info(device: &Peripheral) -> anyhow::Result<Info> {
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

    Ok(info)
}
