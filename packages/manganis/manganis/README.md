# Manganis

The Manganis allows you to submit assets to any build tool that supports collecting assets. It makes it easy to self-host assets that are distributed throughout your libraries. Manganis also handles optimizing, converting, and fetching assets.

If you defined this in a component library:

```rust
use manganis::{Asset, asset};
const AVIF_ASSET: Asset = manganis::asset!("/assets/image.png");
```

AVIF_ASSET will be set to a new file name that will be served by some CLI. That file can be collected by any package that depends on the component library.

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
