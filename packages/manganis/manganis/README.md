# Manganis

Manganis is a tool for submitting assets and native source files to the program linker. It makes it easy to self-host assets and native plugins that are distributed throughout your libraries.

## Assets

Including assets is as simple as using the `asset!()` macro:

```rust
use manganis::{Asset, asset};
const STYLE: Asset = asset!("/assets/style.css");
```

After cargo builds your app, the asset path is embedded directly in the data section. A tool like the Dioxus CLI can extract this metadata and post-process these assets.

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

## option_asset

If you have assets that may not always be bundled, you can fall back gracefully with `option_asset!`:

```rust
use manganis::{Asset, asset, option_asset};
const REQUIRED: Asset = asset!("/assets/style.css");
const OPTIONAL: Option<Asset> = option_asset!("/assets/missing.css");
```

## Native FFI Bindings

Manganis provides the `#[ffi]` attribute macro for generating direct FFI bindings between Rust and native platforms (Swift/Kotlin). See the `geolocation-native-plugin` example for usage.
