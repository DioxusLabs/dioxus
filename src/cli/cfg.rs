use std::path::PathBuf;
use structopt::StructOpt;

use serde::Deserialize;
use std::collections::HashMap;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, StructOpt)]
pub struct ConfigOptsBuild {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[structopt(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Build in release mode [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub release: bool,

    /// Optional pattern for the app loader script [default: None]
    ///
    /// Patterns should include the sequences `{base}`, `{wasm}`, and `{js}` in order to
    /// properly load the application. Other sequences may be included corresponding
    /// to key/value pairs provided in `pattern_params`.
    ///
    /// These values can only be provided via config file.
    #[structopt(skip)]
    #[serde(default)]
    pub pattern_script: Option<String>,

    /// Optional pattern for the app preload element [default: None]
    ///
    /// Patterns should include the sequences `{base}`, `{wasm}`, and `{js}` in order to
    /// properly preload the application. Other sequences may be included corresponding
    /// to key/value pairs provided in `pattern_params`.
    ///
    /// These values can only be provided via config file.
    #[structopt(skip)]
    #[serde(default)]
    pub pattern_preload: Option<String>,

    #[structopt(skip)]
    #[serde(default)]
    /// Optional replacement parameters corresponding to the patterns provided in
    /// `pattern_script` and `pattern_preload`.
    ///
    /// When a pattern is being replaced with its corresponding value from this map, if the value is
    /// prefixed with the symbol `@`, then the value is expected to be a file path, and the pattern
    /// will be replaced with the contents of the target file. This allows insertion of some big JSON
    /// state or even HTML files as a part of the `index.html` build.
    ///
    /// Trunk will automatically insert the `base`, `wasm` and `js` key/values into this map. In order
    //// for the app to be loaded properly, the patterns `{base}`, `{wasm}` and `{js}` should be used
    /// in `pattern_script` and `pattern_preload`.
    ///
    /// These values can only be provided via config file.
    pub pattern_params: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Default, Deserialize, StructOpt)]
pub struct ConfigOptsServe {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[structopt(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Build in release mode [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub release: bool,
}

/// Ensure the given value for `--public-url` is formatted correctly.
pub fn parse_public_url(val: &str) -> String {
    let prefix = if !val.starts_with('/') { "/" } else { "" };
    let suffix = if !val.ends_with('/') { "/" } else { "" };
    format!("{}{}{}", prefix, val, suffix)
}
