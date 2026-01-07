use const_serialize_07 as const_serialize;
use const_serialize_08::{ConstStr, SerializeConst};

use crate::{AssetOptions, AssetOptionsBuilder};

/// Options for a WASM web worker sidecar asset.
///
/// Use this to compile a separate Rust crate to WebAssembly and bundle it
/// as a web worker that can run in a separate thread in the browser.
///
/// # Example
///
/// ```rust,ignore
/// use manganis::{asset, Asset};
///
/// // Compile and bundle a background worker
/// static WORKER: Asset = asset!(
///     "/src/worker",
///     WasmWorkerOptions::new()
///         .release(true)
/// );
///
/// // At runtime, spawn the worker
/// fn spawn_worker() {
///     let worker_url = WORKER.resolve();
///     // Use web_sys to create a Worker with this URL
/// }
/// ```
#[derive(
    Debug,
    Eq,
    PartialEq,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    SerializeConst,
    const_serialize::SerializeConst,
    serde::Serialize,
    serde::Deserialize,
)]
#[const_serialize(crate = const_serialize_08)]
pub struct WasmWorkerOptions {
    /// Comma-separated list of features to enable
    features: ConstStr,
    /// Whether to build in release mode
    release: bool,
}

impl Default for WasmWorkerOptions {
    fn default() -> Self {
        Self::default()
    }
}

impl WasmWorkerOptions {
    /// Create a new WASM worker asset builder
    pub const fn new() -> AssetOptionsBuilder<WasmWorkerOptions> {
        AssetOptions::wasm_worker()
    }

    /// Create default WASM worker options
    pub const fn default() -> Self {
        Self {
            features: ConstStr::new(""),
            release: false,
        }
    }

    /// Get the features to enable (comma-separated)
    pub fn features(&self) -> &str {
        self.features.as_str()
    }

    /// Check if the worker should be built in release mode
    pub const fn is_release(&self) -> bool {
        self.release
    }
}

impl AssetOptions {
    /// Create a new WASM worker asset builder
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, AssetOptions};
    /// const _: Asset = asset!("/src/worker", AssetOptions::wasm_worker().release(true));
    /// ```
    pub const fn wasm_worker() -> AssetOptionsBuilder<WasmWorkerOptions> {
        AssetOptionsBuilder::variant(WasmWorkerOptions::default())
    }
}

impl AssetOptionsBuilder<WasmWorkerOptions> {
    /// Set the features to enable (comma-separated)
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, WasmWorkerOptions};
    /// const _: Asset = asset!("/src/worker", WasmWorkerOptions::new().features("simd,threads"));
    /// ```
    pub const fn features(mut self, features: &'static str) -> Self {
        self.variant.features = ConstStr::new(features);
        self
    }

    /// Set whether to build in release mode
    ///
    /// Release builds are optimized and produce smaller WASM files.
    ///
    /// ```rust,ignore
    /// # use manganis::{asset, Asset, WasmWorkerOptions};
    /// const _: Asset = asset!("/src/worker", WasmWorkerOptions::new().release(true));
    /// ```
    pub const fn release(mut self, release: bool) -> Self {
        self.variant.release = release;
        self
    }

    /// Convert the options into options for a generic asset
    pub const fn into_asset_options(self) -> AssetOptions {
        AssetOptions {
            add_hash: true, // Workers benefit from cache busting
            variant: crate::AssetVariant::WasmWorker(self.variant),
        }
    }
}
