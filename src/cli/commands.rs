pub fn print_help() {
    println!("Usage:");
    println!("  lighthouse-rs [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --poweron             Power on all detected Lighthouse devices");
    println!("  --standby             Put all detected Lighthouse devices in standby mode");
    println!("  --scan                Scan for devices");
    println!("  --devices             Return a list of known devices");
    println!("  --json                Output known devices in JSON format");
    println!("  --help                Display help information");
    println!();
    println!("SteamVR Integration:");
    println!("  --register-steamvr    Register lighthouse-rs with SteamVR for automatic power management");
    println!("  --unregister-steamvr  Unregister from SteamVR");
    println!("  --steamvr-started     Called by SteamVR when it starts (powers on lighthouses)");
    println!(
        "  --steamvr-stopped     Called by SteamVR when it exits (puts lighthouses in standby)"
    );
}
