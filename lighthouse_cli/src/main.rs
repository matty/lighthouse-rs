// Llghthouse-rs: A utility for controlling Lighthouse Base Stations
//
// This application provides a command-line interface for controlling SteamVR Lighthouse
// Base Stations via Bluetooth. It allows scanning for devices, turning them on,
// putting them in standby mode, and can be called by external applications to toggle them.

use lighthouse_core::btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use lighthouse_core::btleplug::platform::Manager;
use std::env;
use std::error::Error;
use std::process;
use std::time::Duration;
use tokio::time;

mod cli;
mod tui;

use cli::{
    error_log, log, print_help, CommandResponse, DEVICES_ARG, EXIT_BLUETOOTH_ERROR,
    EXIT_COMMAND_FAILED, EXIT_GENERAL_ERROR, EXIT_NO_DEVICES_FOUND, EXIT_STEAMVR_ERROR, HELP_ARG,
    JSON_OUTPUT_ARG, POWERON_ARG, REGISTER_STEAMVR_ARG, SCAN_ARG, STANDBY_ARG, STEAMVR_STARTED_ARG,
    STEAMVR_STOPPED_ARG, TUI_ARG, UNREGISTER_STEAMVR_ARG,
};
use lighthouse_core::bluetooth::{
    handle_device_command, peripheral_to_device_info, power_on_lighthouses_with_json,
    scan_process_and_save, standby_lighthouses_with_json,
};
use lighthouse_core::bluetooth::{POWERON_COMMAND, STANDBY_COMMAND};
use lighthouse_core::config::{load_devices, load_devices_with_json};
use lighthouse_core::steamvr_integration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let standby_mode = args.contains(&STANDBY_ARG.to_string());
    let poweron_mode = args.contains(&POWERON_ARG.to_string());
    let scan_only = args.contains(&SCAN_ARG.to_string());
    let devices_mode = args.contains(&DEVICES_ARG.to_string());
    let json_output = args.contains(&JSON_OUTPUT_ARG.to_string());
    let help_requested = args.contains(&HELP_ARG.to_string());
    let tui_mode = args.contains(&TUI_ARG.to_string());

    let register_steamvr = args.contains(&REGISTER_STEAMVR_ARG.to_string());
    let unregister_steamvr = args.contains(&UNREGISTER_STEAMVR_ARG.to_string());
    let steamvr_started = args.contains(&STEAMVR_STARTED_ARG.to_string());
    let steamvr_stopped = args.contains(&STEAMVR_STOPPED_ARG.to_string());

    log("Starting lighthouse-rs...", json_output);

    if help_requested || args.len() <= 1 {
        if !json_output {
            print_help();
        } else {
            let response = CommandResponse::success("help", Vec::new());
            println!("{}", serde_json::to_string(&response)?);
        }
        return Ok(());
    }

    // TUI mode takes precedence over other modes and does not support JSON output
    if tui_mode {
        if json_output {
            log("--json is ignored in TUI mode", false);
        }
        return tui::run_tui().await;
    }

    if devices_mode {
        log("Retrieving device information...", json_output);
        handle_devices_command(json_output).await?;
        return Ok(());
    }

    if register_steamvr {
        log("Registering lighthouse-rs with SteamVR...", json_output);
        handle_steamvr_registration(json_output).await?;
        return Ok(());
    }

    if unregister_steamvr {
        log("Unregistering lighthouse-rs from SteamVR...", json_output);
        handle_steamvr_unregistration(json_output).await?;
        return Ok(());
    }

    if steamvr_started {
        log(
            "SteamVR started event detected. Powering on lighthouses...",
            json_output,
        );
        handle_steamvr_started(json_output).await?;
        return Ok(());
    }

    if steamvr_stopped {
        log(
            "SteamVR stopped event detected. Putting lighthouses in standby...",
            json_output,
        );
        handle_steamvr_stopped(json_output).await?;
        return Ok(());
    }

    if scan_only {
        log(
            "Scan-only mode requested. Will scan for devices and save.",
            json_output,
        );
        handle_scan_command(json_output).await?;
        return Ok(());
    }

    if standby_mode && poweron_mode {
        log(
            "Warning: Both --standby and --poweron flags were provided.",
            json_output,
        );
        log(
            "These operations are mutually exclusive. Prioritizing power on command.",
            json_output,
        );
    }

    let command_mode = if poweron_mode {
        POWERON_COMMAND
    } else if standby_mode {
        STANDBY_COMMAND
    } else {
        0xFF // No command
    };

    if command_mode != 0xFF {
        handle_device_command_mode(command_mode, json_output).await?;
    }

    Ok(())
}

async fn handle_devices_command(json_output: bool) -> Result<(), Box<dyn Error>> {
    match load_devices_with_json(json_output) {
        Ok(devices) => {
            if !devices.is_empty() {
                log(
                    &format!("Found {} cached devices", devices.len()),
                    json_output,
                );
                let response =
                    CommandResponse::success("Successfully retrieved device information", devices);
                println!("{}", serde_json::to_string(&response)?);
                return Ok(());
            } else {
                log("No cached devices found. Performing a scan...", json_output);
                match lighthouse_core::bluetooth::scan_process_and_save_with_json(0xFF, json_output)
                    .await
                {
                    Ok(_) => {
                        let devices = load_devices_with_json(json_output).unwrap_or_default();
                        log(
                            &format!("Scan completed. Found {} devices", devices.len()),
                            json_output,
                        );
                        let response = CommandResponse::success(
                            "Successfully scanned and saved device information",
                            devices,
                        );
                        println!("{}", serde_json::to_string(&response)?);
                        return Ok(());
                    }
                    Err(e) => {
                        error_log(&format!("Failed to scan for devices: {}", e), json_output);
                        let response = CommandResponse::error(
                            &format!("Failed to scan for devices: {}", e),
                            EXIT_BLUETOOTH_ERROR,
                        );
                        println!("{}", serde_json::to_string(&response)?);
                        process::exit(EXIT_BLUETOOTH_ERROR);
                    }
                }
            }
        }
        Err(e) => {
            error_log(&format!("Failed to load device cache: {}", e), json_output);
            let response = CommandResponse::error(
                &format!("Failed to load device cache: {}", e),
                EXIT_GENERAL_ERROR,
            );
            println!("{}", serde_json::to_string(&response)?);
            process::exit(EXIT_GENERAL_ERROR);
        }
    }
}

async fn handle_steamvr_registration(json_output: bool) -> Result<(), Box<dyn Error>> {
    match steamvr_integration::register_with_steamvr(false) {
        Ok(_) => {
            log("Successfully registered with SteamVR", json_output);
            if json_output {
                let response =
                    CommandResponse::success("Successfully registered with SteamVR", Vec::new());
                println!("{}", serde_json::to_string(&response)?);
            }
            Ok(())
        }
        Err(e) => {
            error_log(
                &format!("Failed to register with SteamVR: {}", e),
                json_output,
            );
            if json_output {
                let response = CommandResponse::error(
                    &format!("Failed to register with SteamVR: {}", e),
                    EXIT_STEAMVR_ERROR,
                );
                println!("{}", serde_json::to_string(&response)?);
            }
            process::exit(EXIT_STEAMVR_ERROR);
        }
    }
}

async fn handle_steamvr_unregistration(json_output: bool) -> Result<(), Box<dyn Error>> {
    match steamvr_integration::unregister_from_steamvr() {
        Ok(_) => {
            log("Successfully unregistered from SteamVR", json_output);
            if json_output {
                let response =
                    CommandResponse::success("Successfully unregistered from SteamVR", Vec::new());
                println!("{}", serde_json::to_string(&response)?);
            }
            Ok(())
        }
        Err(e) => {
            error_log(
                &format!("Failed to unregister from SteamVR: {}", e),
                json_output,
            );
            if json_output {
                let response = CommandResponse::error(
                    &format!("Failed to unregister from SteamVR: {}", e),
                    EXIT_STEAMVR_ERROR,
                );
                println!("{}", serde_json::to_string(&response)?);
            }
            process::exit(EXIT_STEAMVR_ERROR);
        }
    }
}

async fn handle_steamvr_started(json_output: bool) -> Result<(), Box<dyn Error>> {
    match power_on_lighthouses_with_json(json_output).await {
        Ok(_) => {
            if json_output {
                let devices = load_devices_with_json(json_output).unwrap_or_default();
                let response =
                    CommandResponse::success("Successfully powered on lighthouses", devices);
                println!("{}", serde_json::to_string(&response)?);
            }
            Ok(())
        }
        Err(e) => {
            if json_output {
                let response = CommandResponse::error(
                    &format!("Failed to power on lighthouses: {}", e),
                    EXIT_COMMAND_FAILED,
                );
                println!("{}", serde_json::to_string(&response)?);
            } else {
                error_log(
                    &format!("Failed to power on lighthouses: {}", e),
                    json_output,
                );
            }
            process::exit(EXIT_COMMAND_FAILED);
        }
    }
}

async fn handle_steamvr_stopped(json_output: bool) -> Result<(), Box<dyn Error>> {
    match standby_lighthouses_with_json(json_output).await {
        Ok(_) => {
            if json_output {
                let devices = load_devices_with_json(json_output).unwrap_or_default();
                let response =
                    CommandResponse::success("Successfully put lighthouses in standby", devices);
                println!("{}", serde_json::to_string(&response)?);
            }
            Ok(())
        }
        Err(e) => {
            if json_output {
                let response = CommandResponse::error(
                    &format!("Failed to put lighthouses in standby: {}", e),
                    EXIT_COMMAND_FAILED,
                );
                println!("{}", serde_json::to_string(&response)?);
            } else {
                error_log(
                    &format!("Failed to put lighthouses in standby: {}", e),
                    json_output,
                );
            }
            process::exit(EXIT_COMMAND_FAILED);
        }
    }
}

async fn handle_scan_command(json_output: bool) -> Result<(), Box<dyn Error>> {
    match lighthouse_core::bluetooth::scan_process_and_save_with_json(0xFF, json_output).await {
        Ok(_) => {
            let devices = load_devices_with_json(json_output).unwrap_or_default();
            if json_output {
                let response = CommandResponse::success(
                    "Successfully scanned and saved device information",
                    devices,
                );
                println!("{}", serde_json::to_string(&response)?);
            }
            Ok(())
        }
        Err(e) => {
            if json_output {
                let response = CommandResponse::error(
                    &format!("Failed to scan for devices: {}", e),
                    EXIT_BLUETOOTH_ERROR,
                );
                println!("{}", serde_json::to_string(&response)?);
            } else {
                error_log(&format!("Failed to scan for devices: {}", e), json_output);
            }
            process::exit(EXIT_BLUETOOTH_ERROR);
        }
    }
}

async fn handle_device_command_mode(
    command_mode: u8,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    let cached_devices = match load_devices() {
        Ok(devices) => devices,
        Err(e) => {
            if json_output {
                let response = CommandResponse::error(
                    &format!("Failed to load known devices: {}", e),
                    EXIT_GENERAL_ERROR,
                );
                println!("{}", serde_json::to_string(&response)?);
            } else {
                eprintln!("Failed to load known devices: {}", e);
            }
            process::exit(EXIT_GENERAL_ERROR);
        }
    };

    if !cached_devices.is_empty() {
        log(
            &format!("Found {} known Lighthouse devices:", cached_devices.len()),
            json_output,
        );
        for (i, device) in cached_devices.iter().enumerate() {
            log(
                &format!(
                    "Known device {}: {} ({})",
                    i + 1,
                    device.name,
                    device.address
                ),
                json_output,
            );
        }

        log("Using known devices automatically.", json_output);

        let manager = match Manager::new().await {
            Ok(m) => m,
            Err(e) => {
                if json_output {
                    let response = CommandResponse::error(
                        &format!("Failed to initialize Bluetooth manager: {}", e),
                        EXIT_BLUETOOTH_ERROR,
                    );
                    println!("{}", serde_json::to_string(&response)?);
                } else {
                    eprintln!("Failed to initialize Bluetooth manager: {}", e);
                }
                process::exit(EXIT_BLUETOOTH_ERROR);
            }
        };

        let adapters = match manager.adapters().await {
            Ok(a) => a,
            Err(e) => {
                if json_output {
                    let response = CommandResponse::error(
                        &format!("Failed to get Bluetooth adapters: {}", e),
                        EXIT_BLUETOOTH_ERROR,
                    );
                    println!("{}", serde_json::to_string(&response)?);
                } else {
                    eprintln!("Failed to get Bluetooth adapters: {}", e);
                }
                process::exit(EXIT_BLUETOOTH_ERROR);
            }
        };

        if adapters.is_empty() {
            let error_msg = "No Bluetooth adapters found";
            if json_output {
                let response = CommandResponse::error(error_msg, EXIT_BLUETOOTH_ERROR);
                println!("{}", serde_json::to_string(&response)?);
            } else {
                eprintln!("{}", error_msg);
            }
            process::exit(EXIT_BLUETOOTH_ERROR);
        }

        let adapter = &adapters[0];
        log(
            &format!("Using adapter: {}", adapter.adapter_info().await?),
            json_output,
        );

        // Start a scan to find the known devices
        match adapter.start_scan(ScanFilter::default()).await {
            Ok(_) => {}
            Err(e) => {
                if json_output {
                    let response = CommandResponse::error(
                        &format!("Failed to start Bluetooth scan: {}", e),
                        EXIT_BLUETOOTH_ERROR,
                    );
                    println!("{}", serde_json::to_string(&response)?);
                } else {
                    eprintln!("Failed to start Bluetooth scan: {}", e);
                }
                process::exit(EXIT_BLUETOOTH_ERROR);
            }
        };

        time::sleep(Duration::from_secs(5)).await;

        let peripherals = match adapter.peripherals().await {
            Ok(p) => p,
            Err(e) => {
                if json_output {
                    let response = CommandResponse::error(
                        &format!("Failed to get peripherals: {}", e),
                        EXIT_BLUETOOTH_ERROR,
                    );
                    println!("{}", serde_json::to_string(&response)?);
                } else {
                    eprintln!("Failed to get peripherals: {}", e);
                }
                process::exit(EXIT_BLUETOOTH_ERROR);
            }
        };

        match adapter.stop_scan().await {
            Ok(_) => {}
            Err(e) => {
                log(
                    &format!("Warning: Failed to stop Bluetooth scan: {}", e),
                    json_output,
                );
            }
        };

        let mut lighthouse_devices = Vec::new();

        for peripheral in peripherals.iter() {
            let address = peripheral.address().to_string();

            if cached_devices
                .iter()
                .any(|device| device.address == address)
            {
                lighthouse_devices.push(peripheral.clone());
            }
        }

        if lighthouse_devices.is_empty() {
            log(
                "None of the cached devices were found in the current scan.",
                json_output,
            );

            if json_output {
                let response = CommandResponse::error(
                    "No cached devices found in the current scan",
                    EXIT_NO_DEVICES_FOUND,
                );
                println!("{}", serde_json::to_string(&response)?);
                process::exit(EXIT_NO_DEVICES_FOUND);
            } else {
                log(
                    "Would you like to perform a new scan to find devices? (y/n)",
                    json_output,
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;

                if input.trim().eq_ignore_ascii_case("y") {
                    log("Performing a new scan...", json_output);
                    match scan_process_and_save(command_mode).await {
                        Ok(_) => {
                            let devices = load_devices().unwrap_or_default();
                            if json_output {
                                let response = CommandResponse::success(
                                    "Successfully executed command on new devices",
                                    devices,
                                );
                                println!("{}", serde_json::to_string(&response)?);
                            }
                            return Ok(());
                        }
                        Err(e) => {
                            if json_output {
                                let response = CommandResponse::error(
                                    &format!("Failed to execute command: {}", e),
                                    EXIT_COMMAND_FAILED,
                                );
                                println!("{}", serde_json::to_string(&response)?);
                            }
                            process::exit(EXIT_COMMAND_FAILED);
                        }
                    }
                } else {
                    log("Exiting without performing a new scan.", json_output);
                    if json_output {
                        let response = CommandResponse::error(
                            "User chose not to perform a new scan",
                            EXIT_NO_DEVICES_FOUND,
                        );
                        println!("{}", serde_json::to_string(&response)?);
                    }
                    process::exit(EXIT_NO_DEVICES_FOUND);
                }
            }
        } else {
            log(
                &format!(
                    "Found {} of {} known devices in the current scan",
                    lighthouse_devices.len(),
                    cached_devices.len()
                ),
                json_output,
            );

            match handle_device_command(&lighthouse_devices, command_mode).await {
                Ok(_) => {
                    if json_output {
                        let mut found_devices = Vec::new();
                        for device in lighthouse_devices.iter() {
                            if let Ok(device_info) = peripheral_to_device_info(device).await {
                                found_devices.push(device_info);
                            }
                        }

                        let command_name = if command_mode == STANDBY_COMMAND {
                            "standby"
                        } else {
                            "power on"
                        };

                        let response = CommandResponse::success(
                            &format!(
                                "Successfully sent {} command to {} devices",
                                command_name,
                                found_devices.len()
                            ),
                            found_devices,
                        );
                        println!("{}", serde_json::to_string(&response)?);
                    }
                }
                Err(e) => {
                    if json_output {
                        let response = CommandResponse::error(
                            &format!("Failed to send command to devices: {}", e),
                            EXIT_COMMAND_FAILED,
                        );
                        println!("{}", serde_json::to_string(&response)?);
                    }
                    process::exit(EXIT_COMMAND_FAILED);
                }
            }
        }
    } else {
        log(
            "No known devices found. Performing a scan automatically...",
            json_output,
        );
        match scan_process_and_save(command_mode).await {
            Ok(_) => {
                let devices = load_devices().unwrap_or_default();
                if json_output {
                    let response = CommandResponse::success(
                        "Successfully scanned and executed command",
                        devices,
                    );
                    println!("{}", serde_json::to_string(&response)?);
                }
            }
            Err(e) => {
                if json_output {
                    let response = CommandResponse::error(
                        &format!("Failed to scan and execute command: {}", e),
                        EXIT_COMMAND_FAILED,
                    );
                    println!("{}", serde_json::to_string(&response)?);
                }
                process::exit(EXIT_COMMAND_FAILED);
            }
        }
    }

    Ok(())
}
