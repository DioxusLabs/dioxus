use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebConfig {
    #[serde(default)]
    pub(crate) app: WebAppConfig,

    #[serde(default)]
    pub(crate) proxy: Vec<WebProxyConfig>,

    #[serde(default)]
    pub(crate) watcher: WebWatcherConfig,

    #[serde(default)]
    pub(crate) resource: WebResourceConfig,

    #[serde(default)]
    pub(crate) https: WebHttpsConfig,

    /// Whether to enable pre-compression of assets and wasm during a web build in release mode
    #[serde(default = "false_bool")]
    pub(crate) pre_compress: bool,

    /// The wasm-opt configuration
    #[serde(default)]
    pub(crate) wasm_opt: WasmOptConfig,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            pre_compress: false_bool(),
            app: Default::default(),
            https: Default::default(),
            wasm_opt: Default::default(),
            proxy: Default::default(),
            watcher: Default::default(),
            resource: Default::default(),
        }
    }
}

/// The wasm-opt configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub(crate) struct WasmOptConfig {
    /// The wasm-opt level to use for release builds [default: s]
    /// Options:
    /// - z: optimize aggressively for size
    /// - s: optimize for size
    /// - 1: optimize for speed
    /// - 2: optimize for more for speed
    /// - 3: optimize for even more for speed
    /// - 4: optimize aggressively for speed
    #[serde(default)]
    pub(crate) level: WasmOptLevel,

    /// Keep debug symbols in the wasm file
    #[serde(default = "false_bool")]
    pub(crate) debug: bool,

    /// Keep the wasm name section, useful for profiling and debugging
    ///
    /// Unlike `debug` which preserves DWARF debug symbols (requiring a browser extension to read),
    /// the name section allows tools like `console_error_panic_hook` to print backtraces with
    /// human-readable function names without any browser extension.
    #[serde(default = "false_bool")]
    pub(crate) keep_names: bool,

    /// Enable memory packing
    #[serde(default = "false_bool")]
    pub(crate) memory_packing: bool,

    /// Extra arguments to pass to wasm-opt
    ///
    /// For example, to enable simd, you can set this to `["--enable-simd"]`.
    ///
    /// You can also disable features by prefixing them with `--disable-`, e.g. `["--disable-bulk-memory"]`.
    ///
    /// Currently only --enable and --disable flags are supported.
    #[serde(default)]
    pub(crate) extra_features: Vec<String>,
}

/// The wasm-opt level to use for release web builds [default: Z]
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) enum WasmOptLevel {
    /// Optimize aggressively for size
    #[serde(rename = "z")]
    #[default]
    Z,
    /// Optimize for size
    #[serde(rename = "s")]
    S,
    /// Don't optimize
    #[serde(rename = "0")]
    Zero,
    /// Optimize for speed
    #[serde(rename = "1")]
    One,
    /// Optimize for more for speed
    #[serde(rename = "2")]
    Two,
    /// Optimize for even more for speed
    #[serde(rename = "3")]
    Three,
    /// Optimize aggressively for speed
    #[serde(rename = "4")]
    Four,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebAppConfig {
    #[serde(default = "default_title")]
    pub(crate) title: String,
    pub(crate) base_path: Option<String>,
}

impl Default for WebAppConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            base_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebProxyConfig {
    pub(crate) backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebWatcherConfig {
    #[serde(default = "watch_path_default")]
    pub(crate) watch_path: Vec<PathBuf>,

    #[serde(default)]
    pub(crate) reload_html: bool,

    #[serde(default = "true_bool")]
    pub(crate) index_on_404: bool,
}

impl Default for WebWatcherConfig {
    fn default() -> Self {
        Self {
            watch_path: watch_path_default(),
            reload_html: false,
            index_on_404: true,
        }
    }
}

fn watch_path_default() -> Vec<PathBuf> {
    vec![PathBuf::from("src"), PathBuf::from("examples")]
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebResourceConfig {
    pub(crate) dev: WebDevResourceConfig,
    pub(crate) style: Option<Vec<PathBuf>>,
    pub(crate) script: Option<Vec<PathBuf>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebDevResourceConfig {
    #[serde(default)]
    pub(crate) style: Vec<PathBuf>,
    #[serde(default)]
    pub(crate) script: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct WebHttpsConfig {
    pub(crate) enabled: Option<bool>,
    pub(crate) mkcert: Option<bool>,
    pub(crate) key_path: Option<String>,
    pub(crate) cert_path: Option<String>,
}

fn true_bool() -> bool {
    true
}

fn false_bool() -> bool {
    false
}

pub(crate) fn default_title() -> String {
    "dioxus | ⛺".into()
}
