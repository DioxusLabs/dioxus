use super::*;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsBuild {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[structopt(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Build in release mode [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub release: bool,

    /// Build a example [default: ""]
    #[structopt(long)]
    pub example: Option<String>,

    /// Build platform: support Web & Desktop [default: "web"]
    #[structopt(long, default_value = "web")]
    pub platform: String,
}

#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub struct ConfigOptsServe {
    /// The index HTML file to drive the bundling process [default: index.html]
    #[structopt(parse(from_os_str))]
    pub target: Option<PathBuf>,

    /// Build a example [default: ""]
    #[structopt(long)]
    pub example: Option<String>,

    /// Build in release mode [default: false]
    #[structopt(long)]
    #[serde(default)]
    pub release: bool,

    /// Build platform: support Web & Desktop [default: "web"]
    #[structopt(long, default_value = "web")]
    pub platform: String,
}

/// Ensure the given value for `--public-url` is formatted correctly.
pub fn parse_public_url(val: &str) -> String {
    let prefix = if !val.starts_with('/') { "/" } else { "" };
    let suffix = if !val.ends_with('/') { "/" } else { "" };
    format!("{}{}{}", prefix, val, suffix)
}
