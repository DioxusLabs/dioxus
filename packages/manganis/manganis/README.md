# Manganis

Manganis is a tool for submitting 3rd-party assets and source files to the program linker. It makes it easy to self-host assets and native plugins that are distributed throughout your libraries.

Including assets is as simple as using the `asset!()` macro:

```rust
const AVIF_ASSET: Asset = asset!("/assets/image.png");
```

After cargo builds your app, the asset path is embedded directly in the data section. A tool like the Dioxus CLI can extract this metadata and post-process these assets.

## CLI Integration

Manganis also handles optimizing, converting, and fetching assets.

## Assets

For some asset types


## Source Files

Manganis makes it possible to include a folder of 3rd-party source files. This allows you to bind against other programming languages, system frameworks, and native APIs without needing to write a `build.rs`.

Manganis supports several different types of source files:

- Swift
- Kotlin

Including the

```rust
static GEOLOCATOR: SwiftPlugin = include_swift!("/plugins/plugin.swift");
```

## Manifest Metadata

Manganis allows exporting arbitrary

## option_asset

If you have assets that may not always be bundled, you can fall back gracefully with `option_asset!`:

```rust
use manganis::{Asset, asset, option_asset};
const REQUIRED: Asset = asset!("/assets/style.css");
const OPTIONAL: Option<Asset> = option_asset!("/assets/missing.css");
```

```rust
use manganis::{ImageFormat, Asset, asset, ImageSize, AssetOptions};
// You can collect arbitrary files. Absolute paths are resolved relative to the package root
const _: Asset = asset!("/assets/script.js");

// You can collect images which will be automatically optimized
pub const PNG_ASSET: Asset =
    asset!("/assets/image.png");
// Resize the image at compile time to make the assets smaller
pub const RESIZED_PNG_ASSET: Asset =
    asset!("/assets/image.png", AssetOptions::image().with_size(ImageSize::Manual { width: 52, height: 52 }));
// Or convert the image at compile time to a web friendly format
pub const AVIF_ASSET: Asset = asset!("/assets/image.png", AssetOptions::image().with_format(ImageFormat::Avif));
```

## Adding Support to Your CLI

To add support for your CLI, you need to integrate with the [manganis_cli_support](https://github.com/DioxusLabs/manganis/tree/main/cli-support) crate. This crate provides utilities to collect assets that integrate with the Manganis macro. It makes it easy to integrate an asset collection and optimization system into a build tool.
