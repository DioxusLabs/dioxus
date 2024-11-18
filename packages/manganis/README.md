# Manganis

The Manganis allows you to submit assets to any build tool that supports collecting assets. It makes it easy to self-host assets that are distributed throughout your libraries. Manganis also handles optimizing, converting, and fetching assets.

If you defined this in a component library:

```rust
const AVIF_ASSET: &str = manganis::asset!("rustacean-flat-gesture.png");
```

AVIF_ASSET will be set to a new file name that will be served by some CLI. That file can be collected by any package that depends on the component library.

```rust
// You can include tailwind classes that will be collected into the final binary
const TAILWIND_CLASSES: &str = manganis::classes!("flex flex-col p-5");

// You can also collect arbitrary files. Relative paths are resolved relative to the package root
const _: Asset = manganis::asset!("test-package-dependency/src/asset.txt");

// You can collect images which will be automatically optimized
pub const PNG_ASSET: manganis::ImageAsset =
    manganis::asset!("rustacean-flat-gesture.png");
// Resize the image at compile time to make the assets smaller
pub const RESIZED_PNG_ASSET: manganis::ImageAsset =
    manganis::asset!("rustacean-flat-gesture.png", ImageAssetOptions::new().size(52, 52));
// Or convert the image at compile time to a web friendly format
pub const AVIF_ASSET: Asset = manganis::asset!("rustacean-flat-gesture.png", ImageAssetOptions::new().format(ImageType::Avif));
```

## Adding Support to Your CLI

To add support for your CLI, you need to integrate with the [manganis_cli_support](https://github.com/DioxusLabs/manganis/tree/main/cli-support) crate. This crate provides utilities to collect assets that integrate with the Manganis macro. It makes it easy to integrate an asset collection and optimization system into a build tool.
