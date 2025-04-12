// SteamVR integration module for Lighthouse-rs
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Constants for SteamVR integration
const STEAMVR_VR_PATH_ENV_VAR: &str = "VR_OVERRIDE";
const STEAMVR_MANIFEST_FILENAME: &str = "lighthouse-rs.vrmanifest";
const MANIFEST_TEMPLATE: &str = include_str!("../steamvr/lighthouse-rs.vrmanifest");

/// Gets the path to the SteamVR manifest file in the application directory
pub fn get_manifest_path() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;
    Ok(exe_dir.join("steamvr").join(STEAMVR_MANIFEST_FILENAME))
}

/// Gets the SteamVR installation directory
pub fn get_steamvr_dir() -> Option<PathBuf> {
    // 1) Try OpenVR paths file in LOCALAPPDATA
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        let ovr_paths = Path::new(&local_app_data)
            .join("openvr")
            .join("openvrpaths.vrpath");
        if ovr_paths.exists() {
            if let Ok(contents) = fs::read_to_string(&ovr_paths) {
                if let Ok(json) = serde_json::from_str::<Value>(&contents) {
                    // Runtime can be a string or an array; handle both
                    if let Some(runtime) = json.get("runtime") {
                        if let Some(s) = runtime.as_str() {
                            let p = Path::new(s);
                            if p.exists() {
                                return Some(p.to_path_buf());
                            }
                        } else if let Some(arr) = runtime.as_array() {
                            if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                                let p = Path::new(first);
                                if p.exists() {
                                    return Some(p.to_path_buf());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2) Try environment override (commonly used for OpenVR dev overrides)
    if let Ok(vr_path) = env::var(STEAMVR_VR_PATH_ENV_VAR) {
        // If pointing to a directory with openvrpaths, try to read it; otherwise treat as runtime dir
        let path = Path::new(&vr_path);
        let ovr_paths = path.join("openvrpaths.vrpath");
        if ovr_paths.exists() {
            if let Ok(contents) = fs::read_to_string(&ovr_paths) {
                if let Ok(json) = serde_json::from_str::<Value>(&contents) {
                    if let Some(runtime) = json
                        .get("runtime")
                        .and_then(|v| v.as_array())
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_str())
                    {
                        let p = Path::new(runtime);
                        if p.exists() {
                            return Some(p.to_path_buf());
                        }
                    }
                }
            }
        }
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    // 3) Common SteamVR installation path
    let steam_paths = vec![
        // Steam default installation path on 64-bit Windows
        "C:\\Program Files (x86)\\Steam\\steamapps\\common\\SteamVR",
    ];

    for path_str in steam_paths {
        let path = Path::new(path_str);
        if path.exists() && path.join("bin").join("win64").exists() {
            return Some(path.to_path_buf());
        }
    }

    None
}

/// Registers the application with SteamVR
pub fn register_with_steamvr(force_register: bool) -> Result<(), Box<dyn Error>> {
    // Get the path to our manifest file
    let manifest_path = get_manifest_path()?;

    // Ensure steamvr directory exists and (re)generate manifest from embedded template
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;
    let steamvr_dir = exe_dir.join("steamvr");
    if !steamvr_dir.exists() {
        fs::create_dir_all(&steamvr_dir)?;
    }

    // Build manifest from embedded template and set absolute binary path
    let mut manifest_json: Value = serde_json::from_str(MANIFEST_TEMPLATE)
        .map_err(|e| format!("Failed to parse embedded manifest template: {}", e))?;
    if let Some(apps) = manifest_json
        .get_mut("applications")
        .and_then(|v| v.as_array_mut())
    {
        if let Some(first) = apps.get_mut(0) {
            if let Some(obj) = first.as_object_mut() {
                obj.insert(
                    "binary_path_windows".to_string(),
                    Value::String(exe_path.to_string_lossy().to_string()),
                );
                // Ensure auto_launch so SteamVR starts this helper automatically
                obj.insert("auto_launch".to_string(), Value::Bool(true));
            }
        }
    }
    let manifest_contents = serde_json::to_string_pretty(&manifest_json)?;
    fs::write(&manifest_path, manifest_contents)?;
    println!("Wrote SteamVR manifest to: {}", manifest_path.display());

    // Get the SteamVR directory
    let steamvr_dir = get_steamvr_dir().ok_or("SteamVR installation not found")?;

    // Path to vrpathreg tool
    let vrpathreg_path = steamvr_dir.join("bin").join("win64").join("vrpathreg.exe");

    if !vrpathreg_path.exists() {
        return Err(format!(
            "vrpathreg.exe not found at expected path: {}",
            vrpathreg_path.display()
        )
        .into());
    }

    // Check if already registered (unless force register is enabled)
    if !force_register {
        let output = Command::new(&vrpathreg_path).arg("show").output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("matty.lighthouse-rs") {
            println!("Application is already registered with SteamVR.");
            return Ok(());
        }
    }

    // Register the manifest with SteamVR
    println!("Registering lighthouse-rs with SteamVR...");

    let output = Command::new(&vrpathreg_path)
        .arg("addmanifest")
        .arg(&manifest_path)
        .output()?;

    if output.status.success() {
        println!("Successfully registered lighthouse-rs with SteamVR!");
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to register with SteamVR: {}", error_message).into());
    }

    Ok(())
}

/// Unregisters the application from SteamVR
pub fn unregister_from_steamvr() -> Result<(), Box<dyn Error>> {
    // Get the SteamVR directory
    let steamvr_dir = get_steamvr_dir().ok_or("SteamVR installation not found")?;

    // Path to vrpathreg tool
    let vrpathreg_path = steamvr_dir.join("bin").join("win64").join("vrpathreg.exe");

    if !vrpathreg_path.exists() {
        return Err(format!(
            "vrpathreg.exe not found at expected path: {}",
            vrpathreg_path.display()
        )
        .into());
    }

    // Get the path to our manifest file
    let manifest_path = get_manifest_path()?;

    // Unregister the manifest from SteamVR
    println!("Unregistering lighthouse-rs from SteamVR...");

    let output = Command::new(vrpathreg_path)
        .arg("removemanifest")
        .arg(&manifest_path)
        .output()?;

    if output.status.success() {
        println!("Successfully unregistered lighthouse-rs from SteamVR!");
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to unregister from SteamVR: {}", error_message).into());
    }

    Ok(())
}
