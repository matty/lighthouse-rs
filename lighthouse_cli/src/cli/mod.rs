// CLI module for command handling
mod commands;
mod response;

pub use commands::*;
pub use response::*;

// Command-line argument constants
pub const STANDBY_ARG: &str = "--standby";
pub const POWERON_ARG: &str = "--poweron";
pub const SCAN_ARG: &str = "--scan";
pub const DEVICES_ARG: &str = "--devices";
pub const JSON_OUTPUT_ARG: &str = "--json";
pub const HELP_ARG: &str = "--help";
pub const TUI_ARG: &str = "--tui";

// SteamVR integration command-line arguments
pub const REGISTER_STEAMVR_ARG: &str = "--register-steamvr";
pub const UNREGISTER_STEAMVR_ARG: &str = "--unregister-steamvr";
pub const STEAMVR_STARTED_ARG: &str = "--steamvr-started";
pub const STEAMVR_STOPPED_ARG: &str = "--steamvr-stopped";

/// Conditionally print messages when not in JSON mode
pub fn log(message: &str, json_output: bool) {
    if !json_output {
        println!("{}", message);
    }
}

/// Conditionally print error messages when not in JSON mode
pub fn error_log(message: &str, json_output: bool) {
    if !json_output {
        eprintln!("{}", message);
    }
}
