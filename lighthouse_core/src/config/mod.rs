use crate::logging::log;
use crate::models::DeviceInfo;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

pub const CONFIG_FILENAME: &str = "lighthouse_devices.json";

pub fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
    let base_dirs = directories::BaseDirs::new().ok_or("Failed to get user directories")?;

    let config_dir = base_dirs
        .data_local_dir()
        .join("com.github.matty.lighthouse-manager");

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&config_dir)?;

    Ok(config_dir.join(CONFIG_FILENAME))
}

pub fn save_devices(devices: &Vec<DeviceInfo>) -> Result<(), Box<dyn Error>> {
    save_devices_with_json(devices, false)
}

pub fn save_devices_with_json(
    devices: &Vec<DeviceInfo>,
    json_output: bool,
) -> Result<(), Box<dyn Error>> {
    let config_path = get_config_path()?;
    log(
        &format!("Saving device info to: {}", config_path.display()),
        json_output,
    );

    let json = serde_json::to_string_pretty(devices)?;
    let mut file = File::create(config_path)?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

pub fn load_devices() -> Result<Vec<DeviceInfo>, Box<dyn Error>> {
    load_devices_with_json(false)
}

pub fn load_devices_with_json(json_output: bool) -> Result<Vec<DeviceInfo>, Box<dyn Error>> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    log(
        &format!("Loading device info from: {}", config_path.display()),
        json_output,
    );

    let mut file = File::open(config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let devices: Vec<DeviceInfo> = serde_json::from_str(&contents)?;
    Ok(devices)
}
