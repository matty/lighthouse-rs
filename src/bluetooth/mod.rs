// Bluetooth module for device control and scanning
mod device_control;
mod scanning;

// Re-export public functions
pub use device_control::*;
pub use scanning::*;

// Bluetooth constants
pub const LHB_PREFIX: &str = "LHB";
pub const LIGHTHOUSE_MANUFACTURER_ID: u16 = 1373;

// Lighthouse service and characteristic UUIDs
pub const LIGHTHOUSE_SERVICE_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0x00001523_1212_efde_1523_785feabcd124);
pub const LIGHTHOUSE_CHAR_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0x00001525_1212_efde_1523_785feabcd124);

// Command values
pub const STANDBY_COMMAND: u8 = 0x00;
pub const POWERON_COMMAND: u8 = 0x01;
