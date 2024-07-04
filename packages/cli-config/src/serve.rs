use clap::Parser;

/// Arguments for the serve command
#[derive(Clone, Debug, Parser, serde::Serialize, serde::Deserialize)]
pub struct ServeArguments {
    /// The port the server will run on
    #[clap(long)]
    #[clap(default_value_t = default_port())]
    pub port: u16,
    /// The address the server will run on
    #[clap(long)]
    pub addr: Option<std::net::IpAddr>,

    /// Open the app in the default browser [default: false - unless project or global settings are set]
    #[clap(long)]
    pub open: bool,

    /// Enable full hot reloading for the app [default: true - unless project or global settings are set]
    #[clap(long, group = "release-incompatible")]
    pub hot_reload: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    #[serde(default)]
    pub cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub args: Vec<String>,
}

impl Default for ServeArguments {
    fn default() -> Self {
        Self {
            port: default_port(),
            addr: None,
            open: false,
            hot_reload: None,
            cross_origin_policy: false,
            args: vec![],
        }
    }
}

impl ServeArguments {
    /// Attempt to read the current serve settings from the CLI. This will only be set for the fullstack platform on recent versions of the CLI.
    pub fn from_cli() -> Option<Self> {
        std::env::var(crate::__private::SERVE_ENV)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
    }
}

fn default_port() -> u16 {
    8080
}
