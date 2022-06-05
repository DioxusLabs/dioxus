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

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long)]
    pub platform: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsServe {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[clap(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build in release mode [default: false]
    #[clap(long)]
    #[serde(default)]
    pub release: bool,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long)]
    pub platform: Option<String>,
}

/// Ensure the given value for `--public-url` is formatted correctly.
pub fn parse_public_url(val: &str) -> String {
    let prefix = if !val.starts_with('/') { "/" } else { "" };
    let suffix = if !val.ends_with('/') { "/" } else { "" };
    format!("{}{}{}", prefix, val, suffix)
}
