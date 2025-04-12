# lighthouse-rs

## About

lighthouse-rs is a utility for managing VR Lighthouse Base Stations via Bluetooth.

## Installation

### Prerequisites

- Windows 10+
- Bluetooth adapter

### Steps

1. Download the latest release from the [Releases](https://github.com/matty/Lighthouse-rs/releases) page
2. Extract the zip file to your preferred location
3. Run `lighthouse-rs.exe` with the required CLI args

## Usage

### Command Line Options

For advanced users or automation, the following command line options are available:

| Command     | Description                                                                 |
| ----------- | --------------------------------------------------------------------------- |
| `--poweron` | Power on all detected Lighthouse devices                                    |
| `--standby` | Put all detected Lighthouse devices in standby mode                         |
| `--scan`    | Scan for devices                                                            |
| `--devices` | Return a list of known devices                                              |
| `--json`    | Output known devices in JSON format                                         |
| `--help`    | Display help information                                                    |

Example:

```
lighthouse-rs.exe --poweron
```

### SteamVR Integration

lighthouse-rs can integrate with SteamVR to automatically power on your Lighthouse devices when SteamVR starts and put them in standby mode when SteamVR exits:

| Command                | Description                                                        |
| ---------------------- | ------------------------------------------------------------------ |
| `--register-steamvr`   | Register lighthouse-rs with SteamVR for automatic power management |
| `--unregister-steamvr` | Unregister from SteamVR                                            |
| `--steamvr-started`    | Called by SteamVR when it starts (powers on lighthouses)           |
| `--steamvr-stopped`    | Called by SteamVR when it exits (puts lighthouses in standby)      |

To set up SteamVR integration:

```
lighthouse-rs.exe --register-steamvr
```

## Building from Source

1. Ensure you have Rust and Cargo installed
2. Clone this repository
3. Run `cargo build`
4. The executable will be available in `target/`

## License

This project is licensed under GNU GPLv3.

## Acknowledgements

- [btleplug](https://github.com/deviceplug/btleplug) library for Bluetooth functionality
