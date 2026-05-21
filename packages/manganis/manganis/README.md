# Manganis

Manganis is a tool for submitting assets and native source files to the program linker. It makes it easy to self-host assets and native plugins that are distributed throughout your libraries.

## Assets

Including assets is as simple as using the `asset!()` macro:

```rust
use manganis::{Asset, asset};
const STYLE: Asset = asset!("/assets/style.css");
```

After cargo builds your app, the asset path is embedded directly in the data section. A tool like the Dioxus CLI can extract this metadata and post-process these assets.

```rust, ignore
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

## JavaScript assets

The CLI auto-detects whether each `.js` asset is an ES module (top-level
`import`/`export` or `import.meta`) or a classic script and processes it
accordingly.

- **`with_minify(true)` (default)**: the file is minified in place. Classic
  scripts are minified verbatim with their format preserved (no IIFE
  re-wrapping). ES modules are minified and bundled — local relative imports
  are inlined into a single ESM file, while `http://` / `https://` imports
  remain as external `import` statements for the browser to resolve at runtime.
- **`with_minify(false)`**: the file is copied byte-for-byte. Use this when
  shipping pre-built third-party libraries you do not want re-processed.
- **`with_static_head(true)`**: appends a `<script>` tag to the document head
  pointing at the asset. The CLI emits `<script type="module" ...>` when the
  file is detected (or declared) as an ES module, and a classic `<script>`
  otherwise.
- **`with_preload(true)`**: emits a `<link rel="preload" as="script">` for the asset.
- **`with_module(true)`**: forces the file to be treated as an ES module even
  when auto-detection would say otherwise. Useful when you author a
  side-effect-only module without top-level `import`/`export` declarations.
  Files named `*.mjs` are always treated as modules; files named `*.cjs` are
  always treated as classic.

```rust, ignore
use manganis::{asset, Asset, AssetOptions};

// Vendored UMD library: copy verbatim, emitted as a classic <script>.
const SWEETALERT: Asset = asset!(
    "/assets/sweetalert2.all.min.js",
    AssetOptions::js().with_minify(false).with_static_head(true)
);

// Authored ES module with local imports: auto-detected from the top-level
// `import`/`export`, bundled and minified into a single ESM file, emitted as
// `<script type="module">`.
const APP: Asset = asset!(
    "/assets/app.js",
    AssetOptions::js().with_static_head(true)
);
```

## Native FFI Bindings

Manganis provides the `#[ffi]` attribute macro for generating direct FFI bindings between Rust and native platforms (Swift/Kotlin). See the `geolocation-native-plugin` example for usage.
