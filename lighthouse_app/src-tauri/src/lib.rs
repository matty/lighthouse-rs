use lighthouse_core::models::DeviceInfo;
use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(all(windows, feature = "installer"))]
use std::os::windows::process::CommandExt;
#[cfg(all(windows, feature = "installer"))]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[tauri::command]
fn get_devices() -> Result<Vec<DeviceInfo>, String> {
    lighthouse_core::config::load_devices().map_err(|e| e.to_string())
}

/// Clear all saved devices from the configuration file
#[tauri::command]
fn clear_saved_devices() -> Result<(), String> {
    lighthouse_core::config::save_devices(&Vec::new()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_for_devices() -> Result<Vec<DeviceInfo>, String> {
    lighthouse_core::bluetooth::scan_process_and_save_with_json(0xFF, false)
        .await
        .map_err(|e| e.to_string())?;
    lighthouse_core::config::load_devices().map_err(|e| e.to_string())
}

#[tauri::command]
async fn power_on_all() -> Result<Vec<DeviceInfo>, String> {
    lighthouse_core::bluetooth::power_on_lighthouses_with_json(false)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn standby_all() -> Result<Vec<DeviceInfo>, String> {
    lighthouse_core::bluetooth::standby_lighthouses_with_json(false)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_steamvr_status() -> Result<bool, String> {
    lighthouse_core::steamvr_integration::is_registered().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_steamvr_registration(enabled: bool) -> Result<(), String> {
    if enabled {
        lighthouse_core::steamvr_integration::register_with_steamvr(false)
            .map_err(|e| e.to_string())
    } else {
        lighthouse_core::steamvr_integration::unregister_from_steamvr().map_err(|e| e.to_string())
    }
}

#[cfg(feature = "installer")]
fn get_install_path() -> Result<PathBuf, String> {
    let local_appdata = env::var("LOCALAPPDATA")
        .map_err(|_| "Failed to get LOCALAPPDATA environment variable".to_string())?;
    Ok(PathBuf::from(local_appdata)
        .join("Programs")
        .join("Lighthouse Manager"))
}

#[tauri::command]
fn check_installation_status() -> Result<bool, String> {
    #[cfg(feature = "installer")]
    {
        let install_path = get_install_path()?;
        let exe_path = install_path.join("Lighthouse Manager.exe");
        Ok(exe_path.exists())
    }
    #[cfg(not(feature = "installer"))]
    {
        Ok(true)
    }
}

#[tauri::command]
fn is_installer_supported() -> bool {
    #[cfg(feature = "installer")]
    {
        true
    }
    #[cfg(not(feature = "installer"))]
    {
        false
    }
}

/// Create a Windows shortcut (.lnk file) using PowerShell
#[cfg(all(windows, feature = "installer"))]
fn create_shortcut(
    shortcut_path: &PathBuf,
    target_path: &PathBuf,
    description: &str,
) -> Result<(), String> {
    use std::process::Command;

    let ps_script = format!(
        r#"
        $WshShell = New-Object -comObject WScript.Shell
        $Shortcut = $WshShell.CreateShortcut("{}")
        $Shortcut.TargetPath = "{}"
        $Shortcut.Description = "{}"
        $Shortcut.Save()
        "#,
        shortcut_path.display(),
        target_path.display(),
        description
    );

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &ps_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to create shortcut: {}", stderr));
    }

    Ok(())
}

#[cfg(all(not(windows), feature = "installer"))]
fn create_shortcut(
    _shortcut_path: &PathBuf,
    _target_path: &PathBuf,
    _description: &str,
) -> Result<(), String> {
    // Shortcuts only supported on Windows
    Ok(())
}

#[tauri::command]
fn install_application(create_desktop_shortcut: bool) -> Result<(), String> {
    #[cfg(feature = "installer")]
    {
        let install_path = get_install_path()?;

        // Create installation directory
        fs::create_dir_all(&install_path)
            .map_err(|e| format!("Failed to create installation directory: {}", e))?;

        // Get current executable path
        let current_exe = env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;

        // Copy executable to install location
        let dest_exe = install_path.join("Lighthouse Manager.exe");
        fs::copy(&current_exe, &dest_exe)
            .map_err(|e| format!("Failed to copy executable: {}", e))?;

        // Create Start Menu shortcut
        let appdata = env::var("APPDATA")
            .map_err(|_| "Failed to get APPDATA environment variable".to_string())?;
        let start_menu_path = PathBuf::from(&appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");

        // Ensure Start Menu Programs folder exists
        fs::create_dir_all(&start_menu_path)
            .map_err(|e| format!("Failed to create Start Menu directory: {}", e))?;

        let start_menu_shortcut = start_menu_path.join("Lighthouse Manager.lnk");
        create_shortcut(&start_menu_shortcut, &dest_exe, "Lighthouse Manager")?;

        // Create Desktop shortcut if requested
        if create_desktop_shortcut {
            let userprofile = env::var("USERPROFILE")
                .map_err(|_| "Failed to get USERPROFILE environment variable".to_string())?;
            let desktop_path = PathBuf::from(&userprofile).join("Desktop");
            let desktop_shortcut = desktop_path.join("Lighthouse Manager.lnk");
            create_shortcut(&desktop_shortcut, &dest_exe, "Lighthouse Manager")?;
        }

        Ok(())
    }
    #[cfg(not(feature = "installer"))]
    {
        let _ = create_desktop_shortcut;
        Err("Installation is not supported in this build.".to_string())
    }
}

#[tauri::command]
fn uninstall_application() -> Result<(), String> {
    #[cfg(feature = "installer")]
    {
        use std::process::Command;

        // Get current executable path
        let current_exe = env::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?;

        // Copy the exe to TEMP directory with a unique name
        let temp_dir =
            env::var("TEMP").map_err(|_| "Failed to get TEMP environment variable".to_string())?;
        let uninstaller_path = PathBuf::from(&temp_dir).join("lighthouse_uninstaller.exe");

        // Copy current executable to temp
        fs::copy(&current_exe, &uninstaller_path)
            .map_err(|e| format!("Failed to copy uninstaller: {}", e))?;

        // Launch the uninstaller with --uninstall flag
        #[cfg(windows)]
        {
            Command::new(&uninstaller_path)
                .arg("--uninstall")
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| format!("Failed to launch uninstaller: {}", e))?;
        }

        #[cfg(not(windows))]
        {
            Command::new(&uninstaller_path)
                .arg("--uninstall")
                .spawn()
                .map_err(|e| format!("Failed to launch uninstaller: {}", e))?;
        }

        // Exit the application so the uninstaller can delete the files
        std::process::exit(0);
    }
    #[cfg(not(feature = "installer"))]
    {
        Err("Uninstallation is not supported in this build.".to_string())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub do_not_show_install_prompt: bool,
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            do_not_show_install_prompt: false,
            theme: "dark".to_string(),
        }
    }
}

fn get_app_config_path() -> Result<PathBuf, String> {
    let local_appdata = env::var("LOCALAPPDATA")
        .map_err(|_| "Failed to get LOCALAPPDATA environment variable".to_string())?;
    let config_dir = PathBuf::from(local_appdata).join("com.github.matty.lighthouse-manager");

    // Ensure directory exists
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(config_dir.join("app.config"))
}

#[tauri::command]
fn get_app_config() -> Result<AppConfig, String> {
    let config_path = get_app_config_path()?;
    if !config_path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))
}

#[tauri::command]
fn save_app_config(config: AppConfig) -> Result<(), String> {
    let config_path = get_app_config_path()?;
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, content).map_err(|e| format!("Failed to write config file: {}", e))
}

#[tauri::command]
fn get_app_data_dir() -> Result<String, String> {
    let local_appdata = env::var("LOCALAPPDATA")
        .map_err(|_| "Failed to get LOCALAPPDATA environment variable".to_string())?;
    let config_dir = PathBuf::from(local_appdata).join("com.github.matty.lighthouse-manager");
    Ok(config_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn reset_application_data() -> Result<(), String> {
    let local_appdata = env::var("LOCALAPPDATA")
        .map_err(|_| "Failed to get LOCALAPPDATA environment variable".to_string())?;
    let config_dir = PathBuf::from(local_appdata).join("com.github.matty.lighthouse-manager");

    if config_dir.exists() {
        for entry in fs::read_dir(&config_dir)
            .map_err(|e| format!("Failed to read config directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let file_name = entry.file_name();

            if file_name != "EBWebView" {
                if path.is_dir() {
                    fs::remove_dir_all(&path)
                        .map_err(|e| format!("Failed to remove directory {:?}: {}", path, e))?;
                } else {
                    fs::remove_file(&path)
                        .map_err(|e| format!("Failed to remove file {:?}: {}", path, e))?;
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
fn restart_application(app: tauri::AppHandle) {
    app.restart();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri::Manager;
            let window = app.get_webview_window("main").unwrap();
            let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize {
                width: 600.0,
                height: 600.0,
            })));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            clear_saved_devices,
            scan_for_devices,
            power_on_all,
            standby_all,
            get_steamvr_status,
            set_steamvr_registration,
            check_installation_status,
            is_installer_supported,
            install_application,
            uninstall_application,
            get_app_config,
            save_app_config,
            get_app_data_dir,
            reset_application_data,
            restart_application
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
