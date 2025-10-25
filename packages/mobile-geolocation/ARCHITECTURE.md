# Architecture: Dioxus Mobile Geolocation

## Overview

This crate demonstrates how to integrate platform-specific code (Java/Swift) into a Rust mobile app with automatic manifest management.

## Current Approach

### What We're Doing
1. **Compile Java → DEX**: Use `android-build` to compile Java shim to DEX bytecode
2. **Embed DEX in Rust**: Use `include_bytes!` to embed compiled DEX
3. **Runtime Loading**: Use `InMemoryDexClassLoader` to load DEX at runtime
4. **JNI Bridge**: Register native methods to call Rust from Java
5. **Permissions**: Declare permissions via `permission!()` macro (auto-injected by CLI)

### The Problem

We're compiling Java/Swift on behalf of the user, but the CLI doesn't know to:
- Copy the `classes.dex` file into the Android APK
- Copy any Swift frameworks into the iOS bundle
- Manage Gradle dependencies

## Alternative: Metadata-Only Approach

### The Insight

Instead of compiling platform shims in `build.rs`, we could:

1. **Export Metadata**: Use linker symbols to export configuration (like permissions already do)
2. **CLI Templating**: Have the CLI generate the Java/Swift shims as part of project generation
3. **Dynamic Compilation**: Let Gradle/Xcode compile the shims

### Example: Configuration Linker Symbols

```rust
// Declare shim requirements via linker symbols
#[export_name = "__SHIM__android_libs"]
static ANDROID_LIBS: &[u8] = b"com.dioxus.geoloc.LocationCallback\0";

#[export_name = "__SHIM__ios_frameworks"]  
static IOS_FRAMEWORKS: &[u8] = b"CoreLocation\0";
```

### CLI Responsibilities

The CLI would:
1. Extract shim metadata from linker symbols
2. Generate Java/Swift files in the Android/iOS project
3. Let the platform build system compile them (Gradle/Xcode)

### Pros
- ✅ No compiling Java/Swift in Rust build
- ✅ Gradle handles Java compilation correctly
- ✅ Xcode handles Swift compilation correctly
- ✅ Simpler build.rs (just metadata embedding)
- ✅ No DEX embedding issues

### Cons
- ❌ More complex CLI (needs to generate Java/Swift)
- ❌ Couples CLI to shim implementations
- ❌ Less control over compilation flags

## Comparison: robius-location

Robius-location compiles Java in `build.rs` using `android-build`:
- ✅ Works reliably (no Gradle issues)
- ✅ Self-contained (no CLI changes needed)
- ✅ Full control over compilation
- ❌ Requires Java compiler in Rust build
- ❌ Generates artifacts that need packaging

## Recommendation

For Dioxus, the **metadata-based approach** makes more sense because:

1. **Dioxus already generates platforms**: The CLI creates Android/iOS projects
2. **CLI handles templates**: Already injects manifests, configs, etc.
3. **Better separation**: Library declares needs, CLI provides infrastructure
4. **Consistent with permissions**: Same pattern as `permission!()` macro

### Implementation Plan

1. Add `shim!()` macro similar to `permission!()`
2. CLI scans for `__SHIM__*` symbols
3. CLI generates appropriate Java/Swift files
4. Gradle/Xcode compiles them in normal build

This is essentially **asking the CLI to provide the platform shims** based on metadata from the library, rather than the library compiling and bundling them itself.

