use crate::bluetooth::device_control::handle_device_command_with_json;
use crate::bluetooth::{LHB_PREFIX, LIGHTHOUSE_MANUFACTURER_ID};
use crate::config::save_devices;
use crate::logging::{error_log, log};
use crate::models::DeviceInfo;
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use tokio::time;

/// Convert a peripheral to DeviceInfo
pub async fn peripheral_to_device_info(
    peripheral: &Peripheral,
) -> Result<DeviceInfo, Box<dyn Error>> {
    let properties = peripheral.properties().await?;
    let address = peripheral.address().to_string();
    let name = properties
        .as_ref()
        .and_then(|p| p.local_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(DeviceInfo { name, address })
}

/// Scan for devices and save them to cache
#[allow(dead_code)]
pub async fn scan_and_save_devices() -> Result<(), Box<dyn Error>> {
    scan_process_and_save(0xFF).await
}

/// Scan, process results and optionally send a command
pub async fn scan_process_and_save(command_mode: u8) -> Result<(), Box<dyn Error>> {
    // Default to non-JSON output for internal calls
    scan_process_and_save_with_json(command_mode, false).await
}

/// Scan, process results and optionally send a command with JSON output control
pub async fn scan_process_and_save_with_json(
    command_mode: u8,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    // Initialize the Bluetooth manager
    let manager = Manager::new().await?;

    // Get the list of available Bluetooth adapters
    let adapters = manager.adapters().await?;
    if adapters.is_empty() {
        error_log("No Bluetooth adapters found", json_output);
        return Err("No Bluetooth adapters found".into());
    }

    // Use the first adapter
    let adapter = &adapters[0];
    log(
        &format!("Using adapter: {}", adapter.adapter_info().await?),
        json_output,
    );

    // Start scanning for devices with a specified timeout
    log("Scanning for Bluetooth devices...", json_output);
    adapter.start_scan(ScanFilter::default()).await?;

    // Delay to allow time for scanning
    time::sleep(Duration::from_secs(5)).await;

    // Get the list of discovered devices
    let peripherals = adapter.peripherals().await?;

    // Process the scan results and potentially send commands
    process_scan_results_with_json(peripherals, command_mode, json_output).await?;

    // Stop scanning
    adapter.stop_scan().await?;
    log("Scanning completed", json_output);

    Ok(())
}

/// Helper function to process scan results, save devices, and optionally send commands
#[allow(dead_code)]
pub async fn process_scan_results(
    peripherals: Vec<Peripheral>,
    command_mode: u8,
) -> Result<(), Box<dyn Error>> {
    // Default to non-JSON output for internal calls
    process_scan_results_with_json(peripherals, command_mode, false).await
}

/// Helper function to process scan results with JSON output control
pub async fn process_scan_results_with_json(
    peripherals: Vec<Peripheral>,
    command_mode: u8,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    if peripherals.is_empty() {
        log("No devices found", json_output);
        return Ok(());
    }

    log(
        &format!("Found {} devices:", peripherals.len()),
        json_output,
    );

    // Create a vector to store filtered lighthouse base stations
    let mut lighthouse_stations = Vec::new();

    // Print information about each discovered device
    for (i, peripheral) in peripherals.iter().enumerate() {
        let properties = peripheral.properties().await?;
        let address = peripheral.address();
        let name = properties
            .as_ref()
            .and_then(|p| p.local_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        log(
            &format!("Device {}: {} ({})", i + 1, name, address),
            json_output,
        );

        // Check if device matches our filter criteria
        let mut is_lighthouse = false;

        // If available, print additional details
        if let Some(properties) = properties {
            // Display manufacturer data if available
            let manufacturer_data = &properties.manufacturer_data;
            for (id, data) in manufacturer_data.iter() {
                log(&format!("  Manufacturer ID: {}", id), json_output);
                log(&format!("  Manufacturer Data: {:?}", data), json_output);

                // Check if this is a Lighthouse device (matches both name and manufacturer ID)
                if name.starts_with(LHB_PREFIX) && *id == LIGHTHOUSE_MANUFACTURER_ID {
                    is_lighthouse = true;
                }
            }

            // Display services if available
            let services = &properties.services;
            if !services.is_empty() {
                log("  Services:", json_output);
                for service in services {
                    log(&format!("    {}", service), json_output);
                }
            }
        }

        // If this is a lighthouse device, add it to our filtered list
        if is_lighthouse {
            lighthouse_stations.push(peripheral.clone());
        }

        log("", json_output);
    }

    // Display information about the filtered Lighthouse devices
    if lighthouse_stations.is_empty() {
        log("No Lighthouse Base Stations found", json_output);
        return Ok(());
    }

    log(
        &format!(
            "Found {} Lighthouse Base Stations:",
            lighthouse_stations.len()
        ),
        json_output,
    );

    // Create a vector to store the device information for caching
    let mut device_info_list = Vec::new();

    for (i, station) in lighthouse_stations.iter().enumerate() {
        let properties = station.properties().await?;
        let address = station.address();
        let name = properties
            .as_ref()
            .and_then(|p| p.local_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        log(
            &format!("Lighthouse {}: {} ({})", i + 1, name, address),
            json_output,
        );

        // Add to our device info list for caching
        let device_info = peripheral_to_device_info(station).await?;
        device_info_list.push(device_info);
    }

    // Save the device information to the config file
    match save_devices(&device_info_list) {
        Ok(_) => log(
            "Successfully saved device information to config file",
            json_output,
        ),
        Err(e) => log(
            &format!("Failed to save device information: {}", e),
            json_output,
        ),
    }

    // If a command mode is requested (not 0xFF), send the command to the devices
    if command_mode != 0xFF {
        handle_device_command_with_json(&lighthouse_stations, command_mode, json_output).await?;
    }

    Ok(())
}
