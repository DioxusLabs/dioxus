use super::*;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBuild {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[clap(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Build in release mode [default: false]
    #[clap(long)]
    #[serde(default)]
    pub release: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long)]
    pub platform: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsServe {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[clap(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Port of dev server
    #[clap(long)]
    #[clap(default_value_t = 8080)]
    pub port: u16,

    /// Open the app in the default browser [default: false]
    #[clap(long)]
    #[serde(default)]
    pub open: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build in release mode [default: false]
    #[clap(long)]
    #[serde(default)]
    pub release: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long)]
    pub platform: Option<String>,

    /// Build with hot reloading rsx [default: false]
    #[clap(long)]
    #[serde(default)]
    pub hot_reload: bool,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name="cross-origin-policy")]
    #[clap(long)]
    #[serde(default)]
    pub cross_origin_policy: bool,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,
}

/// Ensure the given value for `--public-url` is formatted correctly.
pub fn parse_public_url(val: &str) -> String {
    let prefix = if !val.starts_with('/') { "/" } else { "" };
    let suffix = if !val.ends_with('/') { "/" } else { "" };
    format!("{}{}{}", prefix, val, suffix)
}
