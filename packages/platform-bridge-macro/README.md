# platform-bridge-macro

Procedural macro for declaring Android Java sources with linker-based embedding for Dioxus builds.

## Overview

This crate provides the `android_plugin!()` macro for declaring Android Java sources that need to be compiled into the APK.

## Usage

### Basic Example

```rust
use dioxus_platform_bridge::android_plugin;

// Declare Java sources for Android
#[cfg(target_os = "android")]
dioxus_platform_bridge::android_plugin!(
    package = "dioxus.mobile.geolocation",
    plugin = "geolocation",
    files = ["LocationCallback.java", "PermissionsHelper.java"]
);
```

This generates:
- Linker symbols with `__JAVA_SOURCE__` prefix
- Absolute path embedding for fast file resolution
- Compile-time file existence validation

## Macro Syntax

```rust
android_plugin!(
    package = "<java.package.name>",    // Required: Java package (e.g., "dioxus.mobile.geolocation")
    plugin = "<plugin_id>",              // Required: Plugin identifier (e.g., "geolocation")
    files = ["File1.java", ...]         // Required: Array of Java filenames
);
```

### Parameters

- **package**: The Java package name where the classes will live in the APK
- **plugin**: The plugin identifier for organization and symbol naming
- **files**: Array of Java filenames relative to `src/sys/android/` or `src/android/`

## File Resolution

The macro automatically searches for Java files in these locations (relative to `CARGO_MANIFEST_DIR`):

1. `src/sys/android/` (recommended)
2. `src/android/`
3. Root directory (fallback)

If a file is not found, the macro emits a compile error with details about where it searched.

## How It Works

### Compile Time

1. **Validation**: Checks that Java files exist in common locations
2. **Path Resolution**: Converts relative filenames to absolute paths using `env!("CARGO_MANIFEST_DIR")`
3. **Serialization**: Serializes metadata using `const-serialize`
4. **Linker Section**: Embeds data in `__DATA,__java_source` section with unique symbol name

### Build Time (Dioxus CLI)

1. **Extraction**: Parses binary to find `__JAVA_SOURCE__*` symbols
2. **Path Handling**: Uses embedded absolute paths directly (fast path) or searches workspace (legacy)
3. **Copying**: Copies Java files to Gradle structure: `app/src/main/java/{package}/`
4. **Compilation**: Gradle compiles Java sources to DEX bytecode

The macro uses linker-based binary embedding with compile-time validation, similar to the `static_permission!()` and `asset!()` macros.


## Error Messages

If a file is missing, you'll see:

```
error: Java file 'LocationCallback.java' not found. Searched in:
  - /path/to/crate/src/sys/android/LocationCallback.java
  - /path/to/crate/src/android/LocationCallback.java
  - /path/to/crate/LocationCallback.java
```

## See Also

- [`platform-bridge`](../platform-bridge/): Core utilities for Android and iOS/macOS

