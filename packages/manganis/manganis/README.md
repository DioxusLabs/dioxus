# Manganis

The Manganis allows you to submit assets to any build tool that supports collecting assets. It makes it easy to self-host assets that are distributed throughout your libraries. Manganis also handles optimizing, converting, and fetching assets.

If you defined this in a component library:

```rust
use manganis::{Asset, asset};
const AVIF_ASSET: Asset = manganis::asset!("/assets/image.png");
```

AVIF_ASSET will be set to a new file name that will be served by some CLI. That file can be collected by any package that depends on the component library.

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

## Custom mount paths

By default, bundled assets are emitted under `/assets/…`. You can override the served location with `with_mount_path`, which is especially useful for well-known files:

```rust
use manganis::{asset, Asset, AssetOptions};

const SECURITY_TXT: Asset = asset!(
    "/assets/security.txt",
    AssetOptions::builder()
        .with_hash_suffix(false)
        .with_mount_path("/.well-known")
);
```

When paired with the updated CLI, the file above will be written to `/.well-known/security.txt` in web builds, and requests to that URL will resolve correctly at runtime.

## Adding Support to Your CLI

To add support for your CLI, you need to integrate with the [manganis_cli_support](https://github.com/DioxusLabs/manganis/tree/main/cli-support) crate. This crate provides utilities to collect assets that integrate with the Manganis macro. It makes it easy to integrate an asset collection and optimization system into a build tool.
