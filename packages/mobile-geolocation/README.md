# dioxus-mobile-geolocation

Cross-platform geolocation for Dioxus mobile apps with automatic permission management.

This crate provides geolocation functionality for Android and iOS by compiling platform-specific shims (Kotlin for Android, Swift for iOS) during the build process. Permissions are automatically embedded via linker symbols and injected into platform manifests by the Dioxus CLI.

## Features

- **Automatic permission management**: Permissions are embedded as linker symbols and automatically injected into AndroidManifest.xml and Info.plist by the Dioxus CLI
- **Zero-config manifests**: No manual editing of platform manifests required
- **Kotlin/Swift shims**: Native platform code compiled during `cargo build`
- **Robius-compatible**: Uses `robius-android-env` for Android context/JNI access
- **Feature-gated**: Enable only the permissions you need

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dioxus-mobile-geolocation = { path = "../packages/mobile-geolocation" }
```

## Usage

```rust
use dioxus_mobile_geolocation::last_known_location;

fn app() -> Element {
    rsx! {
        button {
            onclick: move |_| {
                if let Some((lat, lon)) = last_known_location() {
                    println!("Location: {}, {}", lat, lon);
                } else {
                    println!("No location available");
                }
            },
            "Get Location"
        }
    }
}
```

## Features

### Default Features

- `android-kotlin`: Enable Android support with Kotlin shim
- `ios-swift`: Enable iOS support with Swift shim
- `location-coarse`: Request coarse/approximate location permission

### Optional Features

- `location-fine`: Request fine/precise GPS location permission
- `background-location`: Request background location access (Android 10+, iOS)

### Example Feature Configuration

```toml
[dependencies]
dioxus-mobile-geolocation = { 
    path = "../packages/mobile-geolocation",
    default-features = false,
    features = ["android-kotlin", "ios-swift", "location-fine"]
}
```

## Permissions

This crate uses the **linker-based permission system**. When you enable location features, the appropriate permissions are embedded as linker symbols in your binary. The Dioxus CLI automatically:

1. Scans your compiled binary for `__PERMISSION__*` symbols
2. Extracts permission metadata (Android permission names, iOS Info.plist keys)
3. Injects them into platform manifests:
   - **Android**: Adds `<uses-permission>` entries to `AndroidManifest.xml`
   - **iOS/macOS**: Adds usage description keys to `Info.plist`

### Android Permissions

The following permissions are automatically added based on enabled features:

- `location-coarse` → `android.permission.ACCESS_COARSE_LOCATION`
- `location-fine` → `android.permission.ACCESS_FINE_LOCATION`
- `background-location` → `android.permission.ACCESS_BACKGROUND_LOCATION` (Android 10+)

### iOS Info.plist Keys

The following keys are automatically added based on enabled features:

- `location-coarse` → `NSLocationWhenInUseUsageDescription`
- `location-fine` → `NSLocationAlwaysAndWhenInUseUsageDescription`
- `background-location` → `NSLocationAlwaysAndWhenInUseUsageDescription`

The usage description strings are taken from the permission declarations in the crate.

## Runtime Permission Requests

While compile-time permissions are handled automatically, you still need to request permissions at runtime on both platforms.

### Android

```rust
// The Kotlin shim provides a helper method for requesting permissions
// You would typically call this before accessing location:

// Example (pseudocode - actual implementation depends on your app structure):
// GeolocationShim.requestPermission(activity, REQUEST_CODE, fine = true)
```

The Kotlin shim checks permissions before accessing location and returns `None` if permissions are not granted.

### iOS

```swift
// Call this before accessing location (typically in your app startup):
import CoreLocation

let locationManager = CLLocationManager()
locationManager.requestWhenInUseAuthorization()

// For background location:
// locationManager.requestAlwaysAuthorization()
```

The Swift shim provides helper functions:
- `ios_geoloc_request_authorization()` - Request when-in-use authorization
- `ios_geoloc_authorization_status()` - Check current authorization status
- `ios_geoloc_services_enabled()` - Check if location services are enabled

## Platform Implementation Details

### Android (Kotlin)

The Android implementation:
1. Compiles Kotlin code via Gradle during `cargo build`
2. Produces an AAR/JAR file in `$OUT_DIR`
3. Uses JNI to call Kotlin methods from Rust
4. Leverages `robius-android-env` to access Android Activity and JNIEnv

The Kotlin shim (`GeolocationShim.kt`) provides:
- `lastKnown(Activity)` - Get last known location
- `requestPermission(Activity, Int, Boolean)` - Request location permissions

### iOS (Swift)

The iOS implementation:
1. Compiles Swift code via `swift build` during `cargo build`
2. Produces a static library (`libGeolocationShim.a`)
3. Links CoreLocation and Foundation frameworks
4. Exposes C ABI functions via `@_cdecl`

The Swift shim (`GeolocationShim.swift`) provides:
- `ios_geoloc_last_known()` - Get last known location
- `ios_geoloc_request_authorization()` - Request authorization
- `ios_geoloc_authorization_status()` - Check authorization status
- `ios_geoloc_services_enabled()` - Check if services are enabled

## Building

### Android Requirements

- Android SDK with API level 24+
- Gradle 8.2+ (included via wrapper)
- Kotlin 1.9+

The Gradle wrapper is included, so you don't need to install Gradle separately.

### iOS Requirements

- Xcode 14+ with Swift 5.9+
- iOS 13+ SDK
- macOS for building

### Build Process

When you run `cargo build --target aarch64-linux-android` or `cargo build --target aarch64-apple-ios`, the `build.rs` script automatically:

1. Detects the target platform
2. Invokes the appropriate build tool (Gradle or Swift)
3. Copies the built artifacts to `$OUT_DIR`
4. Emits linker directives for Cargo

## Integration with Dioxus CLI

When you build your app with `dx build --platform android` or `dx build --platform ios`, the Dioxus CLI:

1. Compiles your Rust code (which triggers this crate's `build.rs`)
2. Scans the final binary for `__PERMISSION__*` symbols
3. Extracts permission metadata
4. Injects permissions into `AndroidManifest.xml` or `Info.plist`

You don't need to manually edit any platform manifests!

## Gradle Integration (Android)

The built AAR/JAR needs to be included in your Android app. Add this to your `app/build.gradle.kts`:

```kotlin
dependencies {
    implementation(files("libs"))
}
```

Then copy the built AAR to your `android/app/libs/` directory:

```bash
cp target/aarch64-linux-android/release/build/dioxus-mobile-geolocation-*/out/geolocation-shim.aar android/app/libs/
```

The Dioxus CLI may automate this step in the future.

## Troubleshooting

### Android: "Class not found" error

Make sure the AAR is copied to `android/app/libs/` and your `build.gradle.kts` includes `implementation(files("libs"))`.

### iOS: "Symbol not found" error

Ensure the Swift library was built successfully. Check the build output for warnings. You may need to:
- Install Xcode command line tools: `xcode-select --install`
- Set the correct SDK path: `xcode-select --switch /Applications/Xcode.app`

### Permissions not appearing in manifest

Make sure you're building with the Dioxus CLI (`dx build`) which includes the permission extraction step. The linker symbols are only scanned during the final bundle/package step.

## References

This crate follows patterns from:
- [Project Robius android-build](https://github.com/project-robius/android-build) - Build-time Android tooling
- [Project Robius robius-android-env](https://github.com/project-robius/robius-android-env) - Android context/JNI access
- [Tauri plugins workspace](https://github.com/tauri-apps/plugins-workspace) - Plugin layout patterns

## License

MIT OR Apache-2.0

