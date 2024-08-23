use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default)]
    pub app: WebAppConfig,

    #[serde(default)]
    pub proxy: Vec<WebProxyConfig>,

    #[serde(default)]
    pub watcher: WebWatcherConfig,

    #[serde(default)]
    pub resource: WebResourceConfig,

    #[serde(default)]
    pub https: WebHttpsConfig,

    /// Whether to enable pre-compression of assets and wasm during a web build in release mode
    #[serde(default = "true_bool")]
    pub pre_compress: bool,

    /// The wasm-opt configuration
    #[serde(default)]
    pub wasm_opt: WasmOptConfig,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            pre_compress: true_bool(),
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WasmOptConfig {
    /// The wasm-opt level to use for release builds [default: s]
    /// Options:
    /// - z: optimize aggressively for size
    /// - s: optimize for size
    /// - 1: optimize for speed
    /// - 2: optimize for more for speed
    /// - 3: optimize for even more for speed
    /// - 4: optimize aggressively for speed
    #[serde(default)]
    pub level: WasmOptLevel,

    /// Keep debug symbols in the wasm file
    #[serde(default = "false_bool")]
    pub debug: bool,
}

/// The wasm-opt level to use for release web builds [default: 4]
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum WasmOptLevel {
    /// Optimize aggressively for size
    #[serde(rename = "z")]
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
    #[default]
    Four,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppConfig {
    #[serde(default = "default_title")]
    pub title: String,
    pub base_path: Option<String>,
}

impl WebAppConfig {
    /// Get the normalized base path for the application with `/` trimmed from both ends. If the base path is not set, this will return `.`.
    pub fn base_path(&self) -> &str {
        match &self.base_path {
            Some(path) => path.trim_matches('/'),
            None => ".",
        }
    }
}

impl Default for WebAppConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            base_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProxyConfig {
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebWatcherConfig {
    #[serde(default = "watch_path_default")]
    pub watch_path: Vec<PathBuf>,

    #[serde(default)]
    pub reload_html: bool,

    #[serde(default = "true_bool")]
    pub index_on_404: bool,
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WebResourceConfig {
    pub dev: WebDevResourceConfig,
    pub style: Option<Vec<PathBuf>>,
    pub script: Option<Vec<PathBuf>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct WebDevResourceConfig {
    #[serde(default)]
    pub style: Vec<PathBuf>,
    #[serde(default)]
    pub script: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WebHttpsConfig {
    pub enabled: Option<bool>,
    pub mkcert: Option<bool>,
    pub key_path: Option<String>,
    pub cert_path: Option<String>,
}

fn true_bool() -> bool {
    true
}

fn false_bool() -> bool {
    false
}

pub fn default_title() -> String {
    "dioxus | ⛺".into()
}
