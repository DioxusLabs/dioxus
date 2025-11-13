# platform-bridge-macro

Procedural macros for declaring Android and iOS/macOS plugin metadata that the Dioxus CLI collects
from linker symbols.

## Overview

The crate exposes two macros:

- `android_plugin!` — declare a prebuilt Android AAR and optional Gradle dependency strings.
- `ios_plugin!` — declare a Swift Package (path + product) that was linked into the binary.

Each macro serializes its metadata as `SymbolData` and emits it under the `__ASSETS__*` linker
prefix, alongside regular assets and permissions. The CLI already performs a single scan of that
prefix after building the Rust binary, so plugin metadata piggy-backs on the same pipeline.

## Android: `android_plugin!`

```rust
use dioxus_platform_bridge::android_plugin;

#[cfg(all(feature = "metadata", target_os = "android"))]
dioxus_platform_bridge::android_plugin!(
    plugin = "geolocation",
    aar = { env = "DIOXUS_ANDROID_ARTIFACT" },
    deps = ["implementation(\"com.google.android.gms:play-services-location:21.3.0\")"]
);
```

### Parameters

| Name   | Required | Description |
|--------|----------|-------------|
| `plugin` | ✅ | Logical plugin identifier used for grouping in diagnostics. |
| `aar` | ✅ | `{ path = "relative/path.aar" }` or `{ env = "ENV_WITH_PATH" }` to locate the artifact. Paths are resolved relative to `CARGO_MANIFEST_DIR`. |
| `deps` | optional | Array of strings (typically Gradle `implementation(...)` lines) appended verbatim to the generated `build.gradle.kts`. |

The macro resolves the artifact path at compile time, wraps it together with the plugin identifier
and dependency strings in `SymbolData::AndroidArtifact`, and emits it via a linker symbol. No Java
source copying or runtime reflection is involved.

**CLI behaviour:** while bundling (`dx bundle --android`), the CLI collects every
`SymbolData::AndroidArtifact`, copies the referenced `.aar` into `app/libs/`, and makes sure the
Gradle module depends on it plus any extra `deps` strings.

## iOS/macOS: `ios_plugin!`

```rust
use dioxus_platform_bridge::ios_plugin;

#[cfg(all(feature = "metadata", any(target_os = "ios", target_os = "macos")))]
dioxus_platform_bridge::ios_plugin!(
    plugin = "geolocation",
    spm = { path = "ios", product = "GeolocationPlugin" }
);
```

### Parameters

| Name   | Required | Description |
|--------|----------|-------------|
| `plugin` | ✅ | Logical plugin identifier. |
| `spm.path` | ✅ | Relative path (from `CARGO_MANIFEST_DIR`) to the Swift package folder. |
| `spm.product` | ✅ | The SwiftPM product name that was linked into the Rust binary. |

The macro emits `SymbolData::SwiftPackage` entries containing the absolute package path and product
name. The CLI uses those entries as a signal to run `swift-stdlib-tool` and embed the Swift runtime
frameworks when bundling for Apple platforms.

## Implementation Notes

- Serialization uses `dx-macro-helpers` which ensures each record is padded to 4 KB for consistent
  linker output.
- Because all metadata ends up in the shared `__ASSETS__*` section, plugin authors do not need any
  additional build steps—the CLI automatically consumes the data in the same pass as assets and
  permissions.
