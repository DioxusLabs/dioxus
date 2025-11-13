# dioxus-platform-bridge

FFI utilities and plugin metadata for Dioxus mobile platform APIs.

This crate provides common patterns and utilities for implementing mobile platform APIs in Dioxus applications. It handles the boilerplate for JNI (Android) and objc2 (iOS) bindings, build scripts, and platform-specific resource management.

## Features

- **Android Support**: JNI utilities, activity caching, DEX loading, callback registration
- **iOS/macOS Support**: Main thread utilities, manager caching, objc2 integration
- **Metadata System**: Declare Java sources in code (collected by dx CLI)

## Usage

### Android APIs

```rust
use dioxus_platform_bridge::android::with_activity;

// Execute JNI operations with cached activity reference
let result = with_activity(|env, activity| {
    // Your JNI operations here
    Some(42)
});
```

### iOS/macOS APIs

```rust
use dioxus_platform_bridge::darwin::MainThreadCell;
use objc2::MainThreadMarker;

let mtm = MainThreadMarker::new().unwrap();
let cell = MainThreadCell::new();
let value = cell.get_or_init_with(mtm, || "initialized");
```

### Declaring Android Java Sources

No build scripts needed! Declare Java sources for Android:

```rust
use dioxus_platform_bridge::android_plugin;

// Declare Java sources (embedded in binary, collected by dx CLI)
#[cfg(target_os = "android")]
dioxus_platform_bridge::android_plugin!(
    package = "dioxus.mobile.geolocation",
    plugin = "geolocation",
    files = ["src/android/LocationCallback.java", "src/android/PermissionsHelper.java"]
);
```

### Declaring iOS/macOS Swift Packages

Declare Swift packages for iOS/macOS builds:

```rust
use dioxus_platform_bridge::ios_plugin;

// Declare Swift package metadata (collected by dx CLI)
#[cfg(any(target_os = "ios", target_os = "macos"))]
dioxus_platform_bridge::ios_plugin!(
    plugin = "geolocation",
    spm = { path = "ios", product = "GeolocationPlugin" }
);
```

## Architecture

The crate is organized into platform-specific modules:

- `android/` - JNI utilities, activity management, callback systems, Java source metadata
- `darwin/` - Main thread utilities for iOS and macOS (objc2)

## License

MIT OR Apache-2.0
