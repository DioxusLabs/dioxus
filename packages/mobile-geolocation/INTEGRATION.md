# Integration Guide

This guide explains how to integrate the `dioxus-mobile-geolocation` crate into your Dioxus mobile application.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Android Integration](#android-integration)
3. [iOS Integration](#ios-integration)
4. [Permission Management](#permission-management)
5. [Runtime Permission Requests](#runtime-permission-requests)
6. [Troubleshooting](#troubleshooting)

## Quick Start

### 1. Add the dependency

```toml
[dependencies]
dioxus-mobile-geolocation = { path = "../packages/mobile-geolocation" }
```

### 2. Use in your app

```rust
use dioxus::prelude::*;
use dioxus_mobile_geolocation::last_known_location;

fn app() -> Element {
    let mut location = use_signal(|| None::<(f64, f64)>);

    rsx! {
        button {
            onclick: move |_| {
                location.set(last_known_location());
            },
            "Get Location"
        }
        
        if let Some((lat, lon)) = location() {
            p { "Latitude: {lat}" }
            p { "Longitude: {lon}" }
        }
    }
}
```

### 3. Build with Dioxus CLI

```bash
# Android
dx build --platform android

# iOS
dx build --platform ios
```

The Dioxus CLI will automatically:
- Compile the Kotlin/Swift shims
- Extract permission symbols from your binary
- Inject permissions into AndroidManifest.xml or Info.plist

## Android Integration

### Build Process

When you build for Android, the following happens automatically:

1. **Gradle Build**: The `build.rs` script invokes Gradle to compile the Kotlin shim
2. **AAR Generation**: Gradle produces an AAR (Android Archive) file
3. **Copy to OUT_DIR**: The AAR is copied to `$OUT_DIR/geolocation-shim.aar`
4. **Permission Injection**: The Dioxus CLI scans your binary and injects permissions

### Manual AAR Integration

If you need to manually integrate the AAR:

1. Find the built AAR:
```bash
find target -name "geolocation-shim.aar"
```

2. Copy it to your Android app's libs directory:
```bash
cp target/aarch64-linux-android/release/build/dioxus-mobile-geolocation-*/out/geolocation-shim.aar \
   android/app/libs/
```

3. Ensure your `app/build.gradle.kts` includes:
```kotlin
dependencies {
    implementation(files("libs"))
}
```

### Android Manifest

The permissions are automatically injected by the Dioxus CLI. You don't need to manually edit `AndroidManifest.xml`.

**Before CLI injection:**
```xml
<manifest>
    <uses-permission android:name="android.permission.INTERNET" />
    <!-- Other permissions -->
</manifest>
```

**After CLI injection (with `location-coarse` feature):**
```xml
<manifest>
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION" />
    <!-- Other permissions -->
</manifest>
```

### Runtime Permission Requests

Android requires runtime permission requests (API 23+). The Kotlin shim provides a helper:

```rust
// Pseudocode - actual implementation depends on your JNI setup
#[cfg(target_os = "android")]
fn request_location_permission() {
    use robius_android_env as aenv;
    use jni::objects::JValue;
    
    let env = aenv::jni_env().unwrap();
    let activity = aenv::activity().unwrap();
    
    let cls = env.find_class("com/dioxus/geoloc/GeolocationShim").unwrap();
    
    // Request fine location (GPS)
    env.call_static_method(
        cls,
        "requestPermission",
        "(Landroid/app/Activity;IZ)V",
        &[
            JValue::Object(&activity.as_obj()),
            JValue::Int(1000), // Request code
            JValue::Bool(1),   // fine = true
        ],
    ).unwrap();
}
```

You should call this before attempting to get location. The user will see a system permission dialog.

## iOS Integration

### Build Process

When you build for iOS, the following happens automatically:

1. **Swift Build**: The `build.rs` script invokes `swift build` to compile the Swift shim
2. **Static Library**: Swift produces `libGeolocationShim.a`
3. **Framework Linking**: The build script emits linker directives for CoreLocation and Foundation
4. **Permission Injection**: The Dioxus CLI scans your binary and injects Info.plist keys

### Info.plist

The usage description keys are automatically injected by the Dioxus CLI. You don't need to manually edit `Info.plist`.

**Before CLI injection:**
```xml
<dict>
    <key>CFBundleName</key>
    <string>MyApp</string>
    <!-- Other keys -->
</dict>
```

**After CLI injection (with `location-coarse` feature):**
```xml
<dict>
    <key>CFBundleName</key>
    <string>MyApp</string>
    <key>NSLocationWhenInUseUsageDescription</key>
    <string>Approximate location for geolocation features</string>
    <!-- Other keys -->
</dict>
```

### Runtime Permission Requests

iOS requires explicit authorization requests. The Swift shim provides helpers:

```rust
#[cfg(target_os = "ios")]
extern "C" {
    fn ios_geoloc_request_authorization();
    fn ios_geoloc_authorization_status() -> i32;
    fn ios_geoloc_services_enabled() -> i32;
}

#[cfg(target_os = "ios")]
fn request_location_permission() {
    unsafe {
        // Check if location services are enabled
        if ios_geoloc_services_enabled() == 0 {
            println!("Location services are disabled");
            return;
        }
        
        // Check current authorization status
        let status = ios_geoloc_authorization_status();
        match status {
            0 => {
                // Not determined - request authorization
                ios_geoloc_request_authorization();
            }
            1 | 2 => {
                // Restricted or denied
                println!("Location access denied");
            }
            3 | 4 => {
                // Already authorized
                println!("Location access granted");
            }
            _ => {}
        }
    }
}
```

Call this early in your app lifecycle, typically in your app's initialization code.

## Permission Management

### Feature Flags

Control which permissions are embedded by enabling/disabling features:

```toml
[dependencies]
dioxus-mobile-geolocation = { 
    path = "../packages/mobile-geolocation",
    default-features = false,
    features = [
        "android-kotlin",      # Enable Android support
        "ios-swift",           # Enable iOS support
        "location-fine",       # Request precise GPS location
        "background-location", # Request background access (optional)
    ]
}
```

### Permission Mapping

| Feature | Android Permission | iOS Info.plist Key |
|---------|-------------------|-------------------|
| `location-coarse` | `ACCESS_COARSE_LOCATION` | `NSLocationWhenInUseUsageDescription` |
| `location-fine` | `ACCESS_FINE_LOCATION` | `NSLocationAlwaysAndWhenInUseUsageDescription` |
| `background-location` | `ACCESS_BACKGROUND_LOCATION` | `NSLocationAlwaysAndWhenInUseUsageDescription` |

### Linker Symbol Embedding

When you enable a feature like `location-coarse`, the crate embeds a linker symbol:

```rust
#[cfg(feature = "location-coarse")]
pub const LOCATION_COARSE: Permission = permission!(
    Location(Coarse),
    description = "Approximate location for geolocation features"
);
```

This generates a `__PERMISSION__<hash>` symbol in your binary containing serialized permission metadata.

### CLI Extraction

The Dioxus CLI extracts these symbols:

1. **Scan Binary**: Uses the `object` crate to parse ELF/Mach-O/PE formats
2. **Find Symbols**: Searches for symbols matching `__PERMISSION__*`
3. **Deserialize**: Reads the serialized `Permission` struct from the binary
4. **Generate Manifests**: Injects platform-specific permission declarations

See `packages/cli/src/build/permissions.rs` for implementation details.

## Runtime Permission Requests

### Best Practices

1. **Request Early**: Ask for permissions when the user first needs them
2. **Explain Why**: Show UI explaining why you need location access
3. **Handle Denial**: Gracefully handle when permissions are denied
4. **Check Status**: Always check permission status before accessing location

### Example Flow

```rust
use dioxus::prelude::*;
use dioxus_mobile_geolocation::last_known_location;

fn app() -> Element {
    let mut location = use_signal(|| None::<(f64, f64)>);
    let mut permission_status = use_signal(|| "unknown");

    rsx! {
        div {
            h1 { "Geolocation Demo" }
            
            // Explain why we need location
            p { "This app needs your location to show nearby places." }
            
            // Request permission button
            button {
                onclick: move |_| {
                    // Platform-specific permission request
                    #[cfg(target_os = "android")]
                    {
                        // Call Android permission request
                        permission_status.set("requesting");
                    }
                    
                    #[cfg(target_os = "ios")]
                    {
                        // Call iOS authorization request
                        permission_status.set("requesting");
                    }
                },
                "Grant Location Permission"
            }
            
            // Get location button (only enabled if permission granted)
            button {
                onclick: move |_| {
                    if let Some(loc) = last_known_location() {
                        location.set(Some(loc));
                    } else {
                        permission_status.set("denied or unavailable");
                    }
                },
                "Get My Location"
            }
            
            // Display location
            if let Some((lat, lon)) = location() {
                div {
                    p { "Latitude: {lat}" }
                    p { "Longitude: {lon}" }
                }
            }
            
            // Display permission status
            p { "Permission: {permission_status}" }
        }
    }
}
```

## Troubleshooting

### Android Issues

#### "Class not found: com/dioxus/geoloc/GeolocationShim"

**Cause**: The AAR is not included in your Android app.

**Solution**:
1. Find the AAR: `find target -name "geolocation-shim.aar"`
2. Copy to libs: `cp <aar-path> android/app/libs/`
3. Verify `build.gradle.kts` includes: `implementation(files("libs"))`

#### "Permission denial: ACCESS_FINE_LOCATION"

**Cause**: Runtime permission not granted.

**Solution**:
1. Request permission using `GeolocationShim.requestPermission()`
2. Handle the permission callback in your Activity
3. Only call `last_known_location()` after permission is granted

#### Gradle build fails

**Cause**: Missing Android SDK or build tools.

**Solution**:
1. Install Android SDK: `sdkmanager "platforms;android-34"`
2. Install build tools: `sdkmanager "build-tools;34.0.0"`
3. Set `ANDROID_HOME` environment variable

### iOS Issues

#### "Symbol not found: _ios_geoloc_last_known"

**Cause**: Swift library not linked.

**Solution**:
1. Check build output for Swift compilation errors
2. Ensure Xcode is installed: `xcode-select --install`
3. Verify Swift toolchain: `swift --version`

#### "This app has crashed because it attempted to access privacy-sensitive data"

**Cause**: Missing Info.plist usage description.

**Solution**:
1. Ensure you're building with `dx build` (not just `cargo build`)
2. Check that Info.plist contains `NSLocationWhenInUseUsageDescription`
3. If missing, the CLI may not have scanned the binary correctly

#### Swift build fails

**Cause**: Incompatible Swift version or SDK.

**Solution**:
1. Update Xcode to latest version
2. Switch to correct Xcode: `sudo xcode-select --switch /Applications/Xcode.app`
3. Clean build: `rm -rf ios-shim/.build`

### Permission Issues

#### Permissions not appearing in manifest

**Cause**: Building with `cargo build` instead of `dx build`.

**Solution**:
- Always use `dx build --platform <android|ios>` for final builds
- The permission extraction only happens during the Dioxus CLI bundle step

#### Wrong permissions injected

**Cause**: Incorrect feature flags.

**Solution**:
1. Check your `Cargo.toml` features
2. Clean build: `cargo clean`
3. Rebuild with correct features

### General Issues

#### Location always returns None

**Possible causes**:
1. Permissions not granted
2. Location services disabled on device
3. No cached location available (device hasn't determined location yet)

**Solutions**:
1. Check permission status
2. Enable location services in device settings
3. Use a location app (Maps) to get an initial fix
4. Wait a few seconds and try again

## Advanced Topics

### Custom Permission Descriptions

You can customize the permission descriptions by forking the crate and modifying the `permission!()` macro calls in `src/lib.rs`:

```rust
#[cfg(feature = "location-coarse")]
pub const LOCATION_COARSE: Permission = permission!(
    Location(Coarse),
    description = "Your custom description here"
);
```

### Multiple Location Precision Levels

You can enable both `location-coarse` and `location-fine` simultaneously. Both permissions will be embedded and injected.

### Background Location

Enable the `background-location` feature for background access:

```toml
features = ["location-fine", "background-location"]
```

On Android 10+, this adds `ACCESS_BACKGROUND_LOCATION` which requires a separate permission request after foreground permission is granted.

On iOS, this uses `NSLocationAlwaysAndWhenInUseUsageDescription` and requires calling `requestAlwaysAuthorization()` instead of `requestWhenInUseAuthorization()`.

## Support

For issues or questions:
- File an issue: https://github.com/DioxusLabs/dioxus/issues
- Discord: https://discord.gg/XgGxMSkvUM
- Documentation: https://dioxuslabs.com/learn/0.6/

