# Dioxus Geolocation Plugin

Get and track the device's current position, including information about altitude, heading, and speed (if available).

| Platform | Supported |
| -------- | --------- |
| Linux    | ✗         |
| Windows  | ✗         |
| macOS    | ✗         |
| Android  | ✓         |
| iOS      | ✓         |

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
dioxus-geolocation = { path = "../path/to/packages/geolocation" }
# or from crates.io when published:
# dioxus-geolocation = "0.7.0-rc.3"
```

## Platform Setup

### iOS

Apple requires privacy descriptions to be specified in `Info.plist` for location information:

- `NSLocationWhenInUseDescription`

### Permissions

This plugin uses the Dioxus permissions crate to declare required permissions. The permissions are automatically embedded in the binary and can be extracted by build tools.

The plugin declares the following permissions:
- **Fine Location**: `ACCESS_FINE_LOCATION` (Android) / `NSLocationWhenInUseUsageDescription` (iOS)
- **Coarse Location**: `ACCESS_COARSE_LOCATION` (Android) / `NSLocationWhenInUseUsageDescription` (iOS)

#### Android

If your app requires GPS functionality to function, add the following to your `AndroidManifest.xml`:

```xml
<uses-feature android:name="android.hardware.location.gps" android:required="true" />
```

The Google Play Store uses this property to decide whether it should show the app to devices without GPS capabilities.

**Note**: The location permissions are automatically added by the Dioxus CLI when building your app, as they are declared using the `permissions` crate.

### Swift Files (iOS/macOS)

This plugin uses the Dioxus platform bridge to declare Swift source files. The Swift files are automatically embedded in the binary and can be extracted by build tools.

The plugin declares the following Swift files:
- `ios/Sources/GeolocationPlugin.swift`

**Note**: Swift files are automatically copied to the iOS/macOS app bundle by the Dioxus CLI when building your app, as they are declared using the `ios_plugin!()` macro.

## Usage

### Basic Example

```rust
use dioxus::prelude::*;
use dioxus_geolocation::{Geolocation, PositionOptions, PermissionState};

fn App() -> Element {
    let mut geolocation = use_signal(|| Geolocation::new());

    rsx! {
        button {
            onclick: move |_| async move {
                // Check permissions
                let status = geolocation.write().check_permissions().unwrap();
                
                if status.location == PermissionState::Prompt {
                    // Request permissions
                    let _ = geolocation.write().request_permissions(None).unwrap();
                }

                // Get current position
                let options = PositionOptions {
                    enable_high_accuracy: true,
                    timeout: 10000,
                    maximum_age: 0,
                };
                
                match geolocation.write().get_current_position(Some(options)) {
                    Ok(position) => {
                        println!("Latitude: {}, Longitude: {}", 
                            position.coords.latitude, 
                            position.coords.longitude
                        );
                    }
                    Err(e) => {
                        eprintln!("Error getting position: {}", e);
                    }
                }
            },
            "Get Current Position"
        }
    }
}
```

### Watching Position Updates

```rust
use dioxus::prelude::*;
use dioxus_geolocation::{Geolocation, PositionOptions, WatchEvent};

fn App() -> Element {
    let mut geolocation = use_signal(|| Geolocation::new());
    let position = use_signal(|| None::<String>);

    rsx! {
        button {
            onclick: move |_| async move {
                let options = PositionOptions {
                    enable_high_accuracy: true,
                    timeout: 10000,
                    maximum_age: 0,
                };

                // Start watching position
                match geolocation.write().watch_position(options, move |event| {
                    match event {
                        WatchEvent::Position(pos) => {
                            let coords = &pos.coords;
                            let msg = format!(
                                "Lat: {:.6}, Lon: {:.6}, Acc: {:.2}m",
                                coords.latitude, coords.longitude, coords.accuracy
                            );
                            position.set(Some(msg));
                            println!("Position update: {:?}", pos);
                        }
                        WatchEvent::Error(err) => {
                            eprintln!("Position error: {}", err);
                            position.set(Some(format!("Error: {}", err)));
                        }
                    }
                }) {
                    Ok(watch_id) => {
                        println!("Started watching position with ID: {}", watch_id);
                        
                        // Later, stop watching:
                        // geolocation.write().clear_watch(watch_id).unwrap();
                    }
                    Err(e) => {
                        eprintln!("Error starting watch: {}", e);
                    }
                }
            },
            "Start Watching Position"
        }

        if let Some(pos_str) = position.read().as_ref() {
            p { "{pos_str}" }
        }
    }
}
```

### Checking and Requesting Permissions

```rust
use dioxus::prelude::*;
use dioxus_geolocation::{Geolocation, PermissionState};

fn App() -> Element {
    let mut geolocation = use_signal(|| Geolocation::new());
    let permission_status = use_signal(|| None::<String>);

    rsx! {
        button {
            onclick: move |_| async move {
                match geolocation.write().check_permissions() {
                    Ok(status) => {
                        let msg = format!(
                            "Location: {:?}, Coarse: {:?}",
                            status.location, status.coarse_location
                        );
                        permission_status.set(Some(msg));
                        
                        if status.location == PermissionState::Prompt {
                            // Request permissions
                            if let Ok(new_status) = geolocation.write().request_permissions(None) {
                                let msg = format!(
                                    "After request - Location: {:?}, Coarse: {:?}",
                                    new_status.location, new_status.coarse_location
                                );
                                permission_status.set(Some(msg));
                            }
                        }
                    }
                    Err(e) => {
                        permission_status.set(Some(format!("Error: {}", e)));
                    }
                }
            },
            "Check Permissions"
        }

        if let Some(status) = permission_status.read().as_ref() {
            p { "{status}" }
        }
    }
}
```

## API Reference

### `Geolocation`

Main entry point for geolocation functionality.

#### Methods

- `new() -> Geolocation` - Create a new Geolocation instance
- `get_current_position(options: Option<PositionOptions>) -> Result<Position>` - Get current position
- `watch_position(options: PositionOptions, callback: F) -> Result<u32>` - Start watching position updates, returns watch ID
- `clear_watch(watch_id: u32) -> Result<()>` - Stop watching position updates
- `check_permissions() -> Result<PermissionStatus>` - Check current permission status
- `request_permissions(permissions: Option<Vec<PermissionType>>) -> Result<PermissionStatus>` - Request permissions

### Types

- `PositionOptions` - Options for getting/watching position
  - `enable_high_accuracy: bool` - Use high accuracy mode (GPS)
  - `timeout: u32` - Maximum wait time in milliseconds
  - `maximum_age: u32` - Maximum age of cached position in milliseconds

- `Position` - Current position data
  - `timestamp: u64` - Timestamp in milliseconds
  - `coords: Coordinates` - Coordinate data

- `Coordinates` - Coordinate information
  - `latitude: f64` - Latitude in decimal degrees
  - `longitude: f64` - Longitude in decimal degrees
  - `accuracy: f64` - Accuracy in meters
  - `altitude: Option<f64>` - Altitude in meters (if available)
  - `altitude_accuracy: Option<f64>` - Altitude accuracy in meters (if available)
  - `speed: Option<f64>` - Speed in m/s (if available)
  - `heading: Option<f64>` - Heading in degrees (if available)

- `PermissionStatus` - Permission status
  - `location: PermissionState` - Location permission state
  - `coarse_location: PermissionState` - Coarse location permission state

- `PermissionState` - Permission state enum
  - `Granted` - Permission granted
  - `Denied` - Permission denied
  - `Prompt` - Permission not yet determined
  - `PromptWithRationale` - Permission prompt with rationale (Android 12+)

- `WatchEvent` - Event from watching position
  - `Position(Position)` - New position update
  - `Error(String)` - Error occurred

## Architecture

This plugin uses Dioxus's platform bridge for Android/iOS integration:

- **Android**: Uses JNI bindings via `dioxus-platform-bridge` to call Kotlin code
- **iOS**: Uses ObjC bindings via `dioxus-platform-bridge` to call Swift code

The native Kotlin/Swift code is designed to be reusable with Tauri plugins, allowing code sharing between Dioxus and Tauri implementations.

## License

MIT OR Apache-2.0
