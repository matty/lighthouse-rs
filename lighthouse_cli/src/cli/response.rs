use lighthouse_core::models::DeviceInfo;
use serde::{Deserialize, Serialize};

// Exit codes for command line interface
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_ERROR: i32 = 1;
pub const EXIT_BLUETOOTH_ERROR: i32 = 2;
pub const EXIT_NO_DEVICES_FOUND: i32 = 3;
pub const EXIT_COMMAND_FAILED: i32 = 4;
pub const EXIT_STEAMVR_ERROR: i32 = 5;

/// Response structure for JSON output
#[derive(Serialize, Deserialize, Debug)]
pub struct CommandResponse {
    pub success: bool,
    pub message: String,
    pub devices: Vec<DeviceInfo>,
    pub error_code: i32,
}

impl CommandResponse {
    /// Create a success response with message and devices
    pub fn success(message: &str, devices: Vec<DeviceInfo>) -> Self {
        CommandResponse {
            success: true,
            message: message.to_string(),
            devices,
            error_code: EXIT_SUCCESS,
        }
    }

    /// Create an error response with message and error code
    pub fn error(message: &str, error_code: i32) -> Self {
        CommandResponse {
            success: false,
            message: message.to_string(),
            devices: Vec::new(),
            error_code,
        }
    }
}
