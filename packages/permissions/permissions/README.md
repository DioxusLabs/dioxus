# Permissions

A cross-platform permission management system with linker-based collection, inspired by Manganis.

This crate provides a unified API for declaring permissions across supported platforms (Android, iOS, macOS) and embeds them in the binary for extraction by build tools.

## Features

- **Cross-platform**: Unified API for all platforms
- **Linker-based collection**: Permissions are embedded in the binary using linker sections
- **Type-safe**: Strongly typed permission kinds, not strings
- **Const-time**: All permission data computed at compile time
- **Extensible**: Support for custom permissions with platform-specific identifiers

## Usage

### Basic Permission Declaration

```rust
use permissions::{static_permission, Permission};

// Declare a camera permission
const CAMERA: Permission = static_permission!(Camera, description = "Take photos");

// Declare a location permission with precision
const LOCATION: Permission = static_permission!(Location(Fine), description = "Track your runs");

// Declare a microphone permission
const MICROPHONE: Permission = static_permission!(Microphone, description = "Record audio");
```

### Custom Permissions (For Untested or Special Use Cases)

For permissions that aren't yet tested or for special use cases, use the `Custom` variant 
with platform-specific identifiers:

```rust
use permissions::{static_permission, Permission};

// Example: Request storage permission
const STORAGE: Permission = static_permission!(
    Custom { 
        android = "android.permission.READ_EXTERNAL_STORAGE",
        ios = "NSPhotoLibraryUsageDescription",
        macos = "NSPhotoLibraryUsageDescription"
    },
    description = "Access files on your device"
);
```

> **💡 Contributing Back**: If you test a custom permission and verify it works across platforms, 
> please consider creating a PR to add it as an officially tested permission! This helps the 
> entire Dioxus community.

### Using Permissions

```rust
use permissions::{static_permission, Permission, Platform};

const CAMERA: Permission = static_permission!(Camera, description = "Take photos");

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
println!("macOS: {:?}", identifiers.macos);
```

## Supported Permission Kinds

Only tested and verified permissions are included. For all other permissions,
use the `Custom` variant.

### ✅ Available Permissions

- **`Camera`** - Camera access (tested on Android, iOS, macOS)
- **`Location(Fine)` / `Location(Coarse)`** - Location access with precision (tested on Android, iOS, macOS)
- **`Microphone`** - Microphone access (tested on Android, iOS, macOS)
- **`Notifications`** - Push notifications (tested on Android, iOS, macOS)
- **`Custom { ... }`** - Custom permission with platform-specific identifiers

For examples of untested permissions (like `PhotoLibrary`, `Contacts`, `Calendar`, `Bluetooth`, etc.),
see the Custom Permissions section below.

## Platform Mappings

Each permission kind automatically maps to the appropriate platform-specific requirements:

| Permission | Android | iOS | macOS |
|------------|---------|-----|-------|
| Camera | `android.permission.CAMERA` | `NSCameraUsageDescription` | `NSCameraUsageDescription` |
| Location(Fine) | `android.permission.ACCESS_FINE_LOCATION` | `NSLocationAlwaysAndWhenInUseUsageDescription` | `NSLocationUsageDescription` |
| Microphone | `android.permission.RECORD_AUDIO` | `NSMicrophoneUsageDescription` | `NSMicrophoneUsageDescription` |

## How It Works

1. **Declaration**: Use the `static_permission!()` macro (or legacy `permission!()`) to declare permissions in your code
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
