# dioxus-platform-bridge

Cross-platform FFI utilities and plugin metadata for Dioxus platform APIs.

This crate provides common patterns and utilities for implementing cross-platform platform APIs in Dioxus applications. It handles the boilerplate for JNI (Android) and objc2 (iOS) bindings, build scripts, and platform-specific resource management.

## Features

- **Android Support**: JNI utilities, activity caching, DEX loading, callback registration
- **iOS Support**: Main thread utilities, manager caching, objc2 integration
- **macOS Support**: Main thread utilities, manager caching, objc2 integration
- **Metadata System**: Declare Java sources and platform frameworks in code (collected by dx CLI)
- **Cross-platform**: Automatic platform detection and appropriate build steps

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

### iOS APIs

```rust
use dioxus_platform_bridge::ios::get_or_init_manager;
use objc2_core_location::CLLocationManager;

// Get or create a manager with main thread safety
let manager = get_or_init_manager(|| {
    unsafe { CLLocationManager::new() }
});
```

### macOS APIs

```rust
use dioxus_platform_bridge::macos::get_or_init_manager;
use objc2_foundation::NSProcessInfo;

// Get or create a manager with main thread safety
let manager = get_or_init_manager(|| {
    unsafe { NSProcessInfo::processInfo() }
});
```

### Declaring Platform Resources

No build scripts needed! Declare Java sources and iOS frameworks in your code:

```rust
use dioxus_platform_bridge::JavaSourceMetadata;

// Declare Java sources (embedded in binary, collected by dx CLI)
#[cfg(target_os = "android")]
const JAVA_SOURCES: JavaSourceMetadata = JavaSourceMetadata::new(
    &["src/android/LocationCallback.java"],
    "com.example.api",
    "example"
);
```

## Architecture

The crate is organized into platform-specific modules:

- `android/` - JNI utilities, activity management, callback systems, Java source metadata
- `ios/` - Main thread utilities, manager caching, iOS framework metadata
- `macos/` - Main thread utilities, manager caching, macOS framework metadata

## Extensibility

This crate now supports:
- **Mobile**: Android (Java/JNI), iOS (objc2)
- **Desktop**: macOS (objc2/Cocoa)
- **Future Support**: Windows API, Linux APIs, Web WASM bindings

The plugin system allows clean declaration of platform-specific resources across all platforms.

## License

MIT OR Apache-2.0
