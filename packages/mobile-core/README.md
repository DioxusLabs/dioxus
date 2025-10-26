# dioxus-mobile-core

Core utilities and abstractions for Dioxus mobile platform APIs.

This crate provides common patterns and utilities for implementing cross-platform mobile APIs in Dioxus applications. It handles the boilerplate for JNI (Android) and objc2 (iOS) bindings, build scripts, and platform-specific resource management.

## Features

- **Android Support**: JNI utilities, activity caching, DEX loading, callback registration
- **iOS Support**: Main thread utilities, manager caching, objc2 integration
- **Build Scripts**: Java→DEX compilation, iOS framework linking
- **Cross-platform**: Automatic platform detection and appropriate build steps

## Usage

### Android APIs

```rust
use dioxus_mobile_core::android::with_activity;

// Execute JNI operations with cached activity reference
let result = with_activity(|env, activity| {
    // Your JNI operations here
    Some(42)
});
```

### iOS APIs

```rust
use dioxus_mobile_core::ios::get_or_init_manager;
use objc2_core_location::CLLocationManager;

// Get or create a manager with main thread safety
let manager = get_or_init_manager(|| {
    unsafe { CLLocationManager::new() }
});
```

### Build Scripts

```rust
// In your build.rs
use dioxus_mobile_core::build::auto_build;
use std::path::PathBuf;

fn main() {
    let java_files = vec![PathBuf::from("src/LocationCallback.java")];
    auto_build(
        &java_files,
        "com.example.api",
        &["CoreLocation", "Foundation"]
    ).unwrap();
}
```

## Architecture

The crate is organized into platform-specific modules:

- `android/` - JNI utilities, activity management, callback systems
- `ios/` - Main thread utilities, manager caching
- `build/` - Build script helpers for Java compilation and framework linking

## License

MIT OR Apache-2.0
