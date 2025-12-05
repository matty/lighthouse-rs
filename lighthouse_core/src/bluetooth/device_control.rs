use crate::bluetooth::{
    LIGHTHOUSE_CHAR_UUID, LIGHTHOUSE_SERVICE_UUID, LHB_PREFIX, LIGHTHOUSE_MANUFACTURER_ID,
    POWERON_COMMAND, STANDBY_COMMAND,
};
use crate::config::save_devices;
use crate::logging::{error_log, log};
use crate::models::DeviceInfo;
use btleplug::api::{
    Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use tokio::time;

/// Send a command to a device
#[allow(dead_code)]
pub async fn send_command_to_device(
    peripheral: &Peripheral,
    command: u8,
) -> Result<(), Box<dyn Error>> {
    send_command_to_device_with_json(peripheral, command, false).await
}

/// Send a command to a device with JSON output control
pub async fn send_command_to_device_with_json(
    peripheral: &Peripheral,
    command: u8,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    let device_name = match peripheral.properties().await? {
        Some(props) => props.local_name.unwrap_or_else(|| "Unknown".to_string()),
        None => "Unknown".to_string(),
    };

    let command_name = match command {
        STANDBY_COMMAND => "standby (0x00)",
        POWERON_COMMAND => "power on (0x01)",
        _ => "unknown",
    };

    log(&format!("Connecting to {}...", device_name), json_output);

    // Connect to the device
    if !peripheral.is_connected().await? {
        peripheral.connect().await?;
        log(&format!("Connected to {}", device_name), json_output);
    } else {
        log(
            &format!("Already connected to {}", device_name),
            json_output,
        );
    }

    // Discover services
    peripheral.discover_services().await?;
    log(
        &format!("Discovered services for {}", device_name),
        json_output,
    );

    // Get device services
    let services = peripheral.services();
    log(
        &format!("Found {} services for {}", services.len(), device_name),
        json_output,
    );

    // Try to find the correct service and characteristic
    // First try with the predefined UUID, if that fails, try to find a writable characteristic
    let mut target_char: Option<Characteristic> = None;

    // Look for our target service UUID first
    for service in services.iter() {
        log(&format!("  Service UUID: {}", service.uuid), json_output);

        // Check if this is our target service or iterate through all
        if service.uuid == LIGHTHOUSE_SERVICE_UUID || target_char.is_none() {
            // Look through all characteristics in this service
            for characteristic in service.characteristics.iter() {
                log(
                    &format!("    Characteristic UUID: {}", characteristic.uuid),
                    json_output,
                );
                log(
                    &format!("    Properties: {:?}", characteristic.properties),
                    json_output,
                );

                // Check if this is our target characteristic or if it has written properties
                if characteristic.uuid == LIGHTHOUSE_CHAR_UUID
                    || (characteristic.properties.contains(CharPropFlags::WRITE)
                        || characteristic
                            .properties
                            .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE))
                {
                    target_char = Some(characteristic.clone());
                    log(
                        &format!("    Found usable characteristic: {}", characteristic.uuid),
                        json_output,
                    );

                    // If this is our exact target, break out
                    if characteristic.uuid == LIGHTHOUSE_CHAR_UUID {
                        break;
                    }
                }
            }

            // If we found our exact target service and characteristic, break out
            if target_char.is_some() && service.uuid == LIGHTHOUSE_SERVICE_UUID {
                break;
            }
        }
    }

    // If we found a writable characteristic, send the command
    if let Some(characteristic) = target_char {
        log(
            &format!("Sending {} command to {}...", command_name, device_name),
            json_output,
        );

        let command_bytes = vec![command];
        peripheral
            .write(&characteristic, &command_bytes, WriteType::WithoutResponse)
            .await?;

        log(
            &format!(
                "{} command sent successfully to {}",
                if command == STANDBY_COMMAND {
                    "Standby"
                } else {
                    "Power on"
                },
                device_name
            ),
            json_output,
        );
    } else {
        log(
            &format!(
                "Could not find a suitable characteristic to write to on {}",
                device_name
            ),
            json_output,
        );
        return Err("No writable characteristic found".into());
    }

    // Disconnect from the device
    peripheral.disconnect().await?;
    log(&format!("Disconnected from {}", device_name), json_output);

    Ok(())
}

/// Handle device commands for multiple devices
pub async fn handle_device_command(
    devices: &[Peripheral],
    command: u8,
) -> Result<(), Box<dyn Error>> {
    // Default to non-JSON output for internal calls
    handle_device_command_with_json(devices, command, false).await
}

/// Handle device commands for multiple devices with JSON output control
pub async fn handle_device_command_with_json(
    devices: &[Peripheral],
    command: u8,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    let command_name = match command {
        STANDBY_COMMAND => "standby",
        POWERON_COMMAND => "power on",
        _ => "unknown operation",
    };

    log(
        &format!(
            "Sending {} command to {} Lighthouse devices...",
            command_name,
            devices.len()
        ),
        json_output,
    );

    for (i, device) in devices.iter().enumerate() {
        log(
            &format!("Processing device {} of {}...", i + 1, devices.len()),
            json_output,
        );

        match send_command_to_device_with_json(device, command, json_output).await {
            Ok(_) => log(
                &format!(
                    "Successfully sent {} command to device {}",
                    command_name,
                    i + 1
                ),
                json_output,
            ),
            Err(e) => log(
                &format!(
                    "Failed to send {} command to device {}: {}",
                    command_name,
                    i + 1,
                    e
                ),
                json_output,
            ),
        }

        // Add a small delay between devices to avoid overwhelming the Bluetooth adapter
        time::sleep(Duration::from_millis(500)).await;
    }

    log(
        &format!("{} operation completed", command_name),
        json_output,
    );
    Ok(())
}

/// Power on lighthouses (called when SteamVR starts)
#[allow(dead_code)]
pub async fn power_on_lighthouses() -> Result<Vec<DeviceInfo>, Box<dyn Error>> {
    // Default to non-JSON output for internal calls
    power_on_lighthouses_with_json(false).await
}

/// Power on lighthouses with JSON output control
/// Returns the list of devices that were found and powered on
pub async fn power_on_lighthouses_with_json(json_output: bool) -> Result<Vec<DeviceInfo>, Box<dyn Error>> {
    log("Powering on lighthouses...", json_output);

    // Initialize Bluetooth
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;

    if adapters.is_empty() {
        error_log("No Bluetooth adapters found", json_output);
        return Err("No Bluetooth adapters found".into());
    }

    let adapter = &adapters[0];
    log(
        &format!("Using adapter: {}", adapter.adapter_info().await?),
        json_output,
    );

    // Start scanning for devices
    log("Scanning for Lighthouse devices...", json_output);
    adapter
        .start_scan(btleplug::api::ScanFilter::default())
        .await?;
    time::sleep(Duration::from_secs(3)).await;

    let peripherals = adapter.peripherals().await?;
    adapter.stop_scan().await?;

    // Find lighthouse devices by checking manufacturer ID and name prefix
    let mut lighthouse_devices = Vec::new();
    let mut device_info_list = Vec::new();

    for peripheral in peripherals.iter() {
        if let Ok(Some(properties)) = peripheral.properties().await {
            let name = properties.local_name.clone().unwrap_or_default();
            
            // Check if this is a lighthouse device
            let is_lighthouse = name.starts_with(LHB_PREFIX) &&
                properties.manufacturer_data.iter().any(|(id, _)| *id == LIGHTHOUSE_MANUFACTURER_ID);
            
            if is_lighthouse {
                lighthouse_devices.push(peripheral.clone());
                device_info_list.push(DeviceInfo {
                    name: name.clone(),
                    address: peripheral.address().to_string(),
                });
                log(&format!("Found lighthouse: {} ({})", name, peripheral.address()), json_output);
            }
        }
    }

    if lighthouse_devices.is_empty() {
        log("No Lighthouse devices found", json_output);
        return Ok(Vec::new());
    }

    log(
        &format!("Found {} Lighthouse devices, saving and powering on...", lighthouse_devices.len()),
        json_output,
    );

    // Save the discovered devices
    if let Err(e) = save_devices(&device_info_list) {
        log(&format!("Failed to save devices: {}", e), json_output);
    }

    // Send the power on command to all found devices
    handle_device_command_with_json(&lighthouse_devices, POWERON_COMMAND, json_output).await?;

    Ok(device_info_list)
}

/// Put lighthouses in standby mode (called when SteamVR stops)
#[allow(dead_code)]
pub async fn standby_lighthouses() -> Result<Vec<DeviceInfo>, Box<dyn Error>> {
    // Default to non-JSON output for internal calls
    standby_lighthouses_with_json(false).await
}

/// Put lighthouses in standby mode with JSON output control
/// Returns the list of devices that were found and put in standby
pub async fn standby_lighthouses_with_json(json_output: bool) -> Result<Vec<DeviceInfo>, Box<dyn Error>> {
    log("Putting lighthouses in standby mode...", json_output);

    // Initialize Bluetooth
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;

    if adapters.is_empty() {
        error_log("No Bluetooth adapters found", json_output);
        return Err("No Bluetooth adapters found".into());
    }

    let adapter = &adapters[0];
    log(
        &format!("Using adapter: {}", adapter.adapter_info().await?),
        json_output,
    );

    // Start scanning for devices
    log("Scanning for Lighthouse devices...", json_output);
    adapter
        .start_scan(btleplug::api::ScanFilter::default())
        .await?;
    time::sleep(Duration::from_secs(3)).await;

    let peripherals = adapter.peripherals().await?;
    adapter.stop_scan().await?;

    // Find lighthouse devices by checking manufacturer ID and name prefix
    let mut lighthouse_devices = Vec::new();
    let mut device_info_list = Vec::new();

    for peripheral in peripherals.iter() {
        if let Ok(Some(properties)) = peripheral.properties().await {
            let name = properties.local_name.clone().unwrap_or_default();
            
            // Check if this is a lighthouse device
            let is_lighthouse = name.starts_with(LHB_PREFIX) &&
                properties.manufacturer_data.iter().any(|(id, _)| *id == LIGHTHOUSE_MANUFACTURER_ID);
            
            if is_lighthouse {
                lighthouse_devices.push(peripheral.clone());
                device_info_list.push(DeviceInfo {
                    name: name.clone(),
                    address: peripheral.address().to_string(),
                });
                log(&format!("Found lighthouse: {} ({})", name, peripheral.address()), json_output);
            }
        }
    }

    if lighthouse_devices.is_empty() {
        log("No Lighthouse devices found", json_output);
        return Ok(Vec::new());
    }

    log(
        &format!("Found {} Lighthouse devices, saving and putting in standby...", lighthouse_devices.len()),
        json_output,
    );

    // Save the discovered devices
    if let Err(e) = save_devices(&device_info_list) {
        log(&format!("Failed to save devices: {}", e), json_output);
    }

    // Send the standby command to all found devices
    handle_device_command_with_json(&lighthouse_devices, STANDBY_COMMAND, json_output).await?;

    Ok(device_info_list)
}
