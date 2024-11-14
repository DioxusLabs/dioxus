#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

pub use const_serialize;

mod folder;
pub use folder::*;

mod images;
pub use images::*;

mod builder;
pub use builder::*;

mod linker;
pub use linker::*;

mod css;
pub use css::*;

mod js;
pub use js::*;

/// The mg macro collects assets that will be included in the final binary
///
/// # Files
///
/// The file builder collects an arbitrary file. Relative paths are resolved relative to the package root
/// ```rust
/// # use manganis::asset;
/// const _: &str = asset!("src/asset.txt");
/// ```
/// Or you can use URLs to read the asset at build time from a remote location
/// ```rust
/// # use manganis::asset;
/// const _: &str = asset!("https://rustacean.net/assets/rustacean-flat-happy.png");
/// ```
///
/// # Images
///
/// You can collect images which will be automatically optimized with the image builder:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png"));
/// ```
/// Resize the image at compile time to make the assets file size smaller:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").size(52, 52));
/// ```
/// Or convert the image at compile time to a web friendly format:
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").format(ImageFormat::Avif).size(52, 52));
/// ```
/// You can mark images as preloaded to make them load faster in your app
/// ```rust
/// # use manganis::asset;
/// const _: manganis::ImageAsset = asset!(image("rustacean-flat-gesture.png").preload());
/// ```
#[macro_export]
macro_rules! asset {
    ($asset:literal $($tokens:tt)*) => {{
        const ASSET: $crate::AssetBuilder = $crate::AssetBuilder::new($asset) $($tokens)*;
        const BUFFER: $crate::const_serialize::ConstWriteBuffer = {
            let write = $crate::const_serialize::ConstWriteBuffer::new();
            $crate::const_serialize::serialize_const(&ASSET, write)
        };
        const BYTES: &[u8] = BUFFER.as_ref();
        const LEN: usize = BYTES.len();

        #[link_section = $crate::__current_link_section!()]
        #[used]
        static LINK_SECTION: [u8; LEN] = {
            let mut bytes = [0; LEN];
            let mut i = 0;
            while i < LEN {
                bytes[i] = BYTES[i];
                i += 1;
            }
            bytes
        };

        ASSET.build()
    }};
}
