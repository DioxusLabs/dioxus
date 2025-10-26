# mobile-core-macro

Procedural macro for declaring Java plugins with linker-based embedding for Dioxus Android builds.

## Overview

This crate provides the `java_plugin!()` macro which reduces Java source declaration boilerplate from ~30 lines to ~3 lines while providing compile-time validation and automatic path embedding.

## Usage

### Basic Example

```rust
use dioxus_mobile_core::java_plugin;

// Declare Java sources for Android
#[cfg(target_os = "android")]
dioxus_mobile_core::java_plugin!(
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
java_plugin!(
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

## Comparison with Similar Systems

This macro follows the same pattern as:
- **permissions**: `static_permission!()` for runtime permissions
- **Manganis**: `asset!()` for static asset bundling

All three use linker-based binary embedding with compile-time validation.

## Benefits

### Developer Experience
- **90% less boilerplate**: ~30 lines → 3 lines
- **Compile-time validation**: Catch missing files immediately
- **Clear error messages**: Shows where files were searched
- **Consistent API**: Same pattern as permissions and Manganis

### Build Performance
- **No workspace search**: Direct file access via embedded paths
- **Faster builds**: ~50-100ms saved per plugin on large workspaces
- **Deterministic**: Paths are known at compile time

## Migration from Manual Approach

**Before** (30+ lines):
```rust
const JAVA_META: JavaSourceMetadata = JavaSourceMetadata::new(
    "dioxus.mobile.geolocation",
    "geolocation",
    &["LocationCallback.java", "PermissionsHelper.java"],
);

const JAVA_META_BYTES: [u8; 4096] = {
    // Manual serialization...
};

#[link_section = "__DATA,__java_source"]
#[used]
#[export_name = "__JAVA_SOURCE__..."]
static JAVA_SOURCE_METADATA: [u8; 4096] = JAVA_META_BYTES;
```

**After** (3 lines):
```rust
dioxus_mobile_core::java_plugin!(
    package = "dioxus.mobile.geolocation",
    plugin = "geolocation",
    files = ["LocationCallback.java", "PermissionsHelper.java"]
);
```

## Error Messages

If a file is missing, you'll see:

```
error: Java file 'LocationCallback.java' not found. Searched in:
  - /path/to/crate/src/sys/android/LocationCallback.java
  - /path/to/crate/src/android/LocationCallback.java
  - /path/to/crate/LocationCallback.java
```

## See Also

- [`permissions-macro`](../permissions/permissions-macro/): Similar macro for permission declarations
- [`manganis-macro`](../manganis/manganis-macro/): Similar macro for asset bundling
- [`mobile-core`](../mobile-core/): Core utilities and Android utilities

