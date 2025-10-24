# Permissions

A cross-platform permission management system with linker-based collection, inspired by Manganis.

This crate provides a unified API for declaring permissions across all platforms (Android, iOS, macOS, Windows, Linux, Web) and embeds them in the binary for extraction by build tools.

## Features

- **Cross-platform**: Unified API for all platforms
- **Linker-based collection**: Permissions are embedded in the binary using linker sections
- **Type-safe**: Strongly typed permission kinds, not strings
- **Const-time**: All permission data computed at compile time
- **Extensible**: Support for custom permissions with platform-specific identifiers

## Usage

### Basic Permission Declaration

```rust
use permissions::permission;

// Declare a camera permission
const CAMERA: Permission = permission!(Camera, description = "Take photos");

// Declare a location permission with precision
const LOCATION: Permission = permission!(Location(Fine), description = "Track your runs");

// Declare a microphone permission
const MICROPHONE: Permission = permission!(Microphone, description = "Record audio");
```

### Custom Permissions

```rust
use permissions::permission;

// Declare a custom permission with platform-specific identifiers
const CUSTOM: Permission = permission!(
    Custom { 
        android = "android.permission.MY_PERMISSION",
        ios = "NSMyUsageDescription",
        macos = "NSMyUsageDescription", 
        windows = "myCapability",
        linux = "my_permission",
        web = "my-permission"
    },
    description = "Custom permission for my app"
);
```

### Using Permissions

```rust
use permissions::{permission, Platform};

const CAMERA: Permission = permission!(Camera, description = "Take photos");

// Get the description
println!("Description: {}", CAMERA.description());

// Check platform support
if CAMERA.supports_platform(Platform::Android) {
    println!("Android permission: {:?}", CAMERA.android_permission());
}

if CAMERA.supports_platform(Platform::Ios) {
    println!("iOS key: {:?}", CAMERA.ios_key());
}

// Get all platform identifiers
let identifiers = CAMERA.platform_identifiers();
println!("Android: {:?}", identifiers.android);
println!("iOS: {:?}", identifiers.ios);
println!("Web: {:?}", identifiers.web);
```

## Supported Permission Kinds

### Cross-Platform Permissions

- `Camera` - Camera access
- `Location(Fine)` / `Location(Coarse)` - Location access with precision
- `Microphone` - Microphone access
- `PhotoLibrary` - Photo library access
- `Contacts` - Contact list access
- `Calendar` - Calendar access
- `Bluetooth` - Bluetooth access
- `Notifications` - Push notifications
- `FileSystem` - File system access
- `Network` - Network access

### Platform-Specific Permissions

#### Android-only
- `Sms` - SMS access
- `PhoneState` - Phone state access
- `PhoneCall` - Phone call access
- `SystemAlertWindow` - System alert window

#### iOS/macOS-only
- `UserTracking` - User tracking
- `FaceId` - Face ID access
- `LocalNetwork` - Local network access

#### Windows-only
- `Appointments` - Appointments access
- `WindowsPhoneCall` - Phone call access
- `EnterpriseAuth` - Enterprise authentication

#### Web-only
- `Clipboard` - Clipboard access
- `Payment` - Payment handling
- `ScreenWakeLock` - Screen wake lock

## Platform Mappings

Each permission kind automatically maps to the appropriate platform-specific requirements:

| Permission | Android | iOS | macOS | Windows | Linux | Web |
|------------|---------|-----|-------|---------|-------|-----|
| Camera | `android.permission.CAMERA` | `NSCameraUsageDescription` | `NSCameraUsageDescription` | `webcam` | None | `camera` |
| Location(Fine) | `android.permission.ACCESS_FINE_LOCATION` | `NSLocationAlwaysAndWhenInUseUsageDescription` | `NSLocationUsageDescription` | `location` | None | `geolocation` |
| Microphone | `android.permission.RECORD_AUDIO` | `NSMicrophoneUsageDescription` | `NSMicrophoneUsageDescription` | `microphone` | None | `microphone` |

## How It Works

1. **Declaration**: Use the `permission!()` macro to declare permissions in your code
2. **Embedding**: The macro embeds permission data in linker sections with `__PERMISSION__*` symbols
3. **Collection**: Build tools can extract permissions by scanning the binary for these symbols
4. **Injection**: Permissions can be injected into platform-specific configuration files

## Build Tool Integration

The embedded `__PERMISSION__*` symbols can be extracted by build tools to:

- Inject permissions into AndroidManifest.xml
- Inject permissions into iOS Info.plist
- Generate permission request code
- Validate permission usage

## Examples

See the `examples/` directory for complete examples of using permissions in different contexts.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
