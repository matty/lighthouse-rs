// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for specific arguments to run in headless mode
    if args.contains(&"--steamvr-started".to_string()) {
        run_headless(true);
        return;
    } else if args.contains(&"--steamvr-stopped".to_string()) {
        run_headless(false);
        return;
    } else if args.contains(&"--uninstall".to_string()) {
        #[cfg(feature = "installer")]
        run_uninstall();
        return;
    }

    vrft_app_lib::run()
}

fn run_headless(power_on: bool) {
    // Create a runtime for async execution
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    rt.block_on(async {
        if power_on {
            println!("Headless: Powering on lighthouses...");
            if let Err(e) = lighthouse_core::bluetooth::power_on_lighthouses_with_json(false).await
            {
                eprintln!("Failed to power on lighthouses: {}", e);
            }
        } else {
            println!("Headless: Setting lighthouses to standby...");
            if let Err(e) = lighthouse_core::bluetooth::standby_lighthouses_with_json(false).await {
                eprintln!("Failed to set lighthouses to standby: {}", e);
            }
        }
    });
}

/// Run the uninstall process
/// This is called when the app is launched with --uninstall
/// It spawns cmd.exe which waits for the main app to exit, then removes all files
#[cfg(feature = "installer")]
fn run_uninstall() {
    #[cfg(windows)]
    {
        use std::process::Command;

        // Get paths for cleanup
        let local_appdata = match env::var("LOCALAPPDATA") {
            Ok(path) => path,
            Err(_) => {
                eprintln!("Failed to get LOCALAPPDATA");
                return;
            }
        };

        let install_path = PathBuf::from(&local_appdata)
            .join("Programs")
            .join("Lighthouse Manager");

        let appdata_config_path =
            PathBuf::from(&local_appdata).join("com.github.matty.lighthouse-manager");

        let start_menu_shortcut = env::var("APPDATA")
            .map(|appdata| {
                PathBuf::from(&appdata)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
                    .join("Lighthouse Manager.lnk")
            })
            .ok();

        let desktop_shortcut = env::var("USERPROFILE")
            .map(|userprofile| {
                PathBuf::from(&userprofile)
                    .join("Desktop")
                    .join("Lighthouse Manager.lnk")
            })
            .ok();

        let current_exe = match env::current_exe() {
            Ok(exe) => exe.to_string_lossy().to_string(),
            Err(_) => return,
        };

        // Build the batch script that will:
        // 1. Wait for the main app to exit
        // 2. Delete installation files
        // 3. Delete itself
        let mut script = String::new();
        script.push_str("@echo off\n");
        script.push_str("echo Waiting for Lighthouse Manager to exit...\n");

        // Wait for the main app process to exit (poll every second, max 30 seconds)
        script.push_str(":waitloop\n");
        script.push_str("tasklist /FI \"IMAGENAME eq Lighthouse Manager.exe\" /NH 2>nul | find /i \"Lighthouse Manager.exe\" >nul\n");
        script.push_str("if %errorlevel%==0 (\n");
        script.push_str("    timeout /t 1 /nobreak >nul\n");
        script.push_str("    goto waitloop\n");
        script.push_str(")\n");
        script.push_str("echo Uninstalling Lighthouse Manager...\n");

        // Remove installation directory
        script.push_str(&format!("rmdir /s /q \"{}\"\n", install_path.display()));

        // Remove AppData config directory
        script.push_str(&format!(
            "rmdir /s /q \"{}\"\n",
            appdata_config_path.display()
        ));

        // Remove shortcuts
        if let Some(shortcut) = start_menu_shortcut {
            script.push_str(&format!("del /f /q \"{}\"\n", shortcut.display()));
        }
        if let Some(shortcut) = desktop_shortcut {
            script.push_str(&format!("del /f /q \"{}\"\n", shortcut.display()));
        }

        // Delete the uninstaller exe (self)
        script.push_str(&format!("del /f /q \"{}\"\n", current_exe));

        script.push_str("echo Uninstall complete.\n");

        // Write script to temp file and execute it
        let temp_dir = env::var("TEMP").unwrap_or_else(|_| ".".to_string());
        let script_path = PathBuf::from(&temp_dir).join("lighthouse_uninstall.bat");

        if fs::write(&script_path, &script).is_ok() {
            // Spawn cmd.exe to run the script
            let _ = Command::new("cmd")
                .args(["/C", &script_path.to_string_lossy()])
                .spawn();
        }
    }

    #[cfg(not(windows))]
    {
        eprintln!("Uninstall is only supported on Windows");
    }
}
