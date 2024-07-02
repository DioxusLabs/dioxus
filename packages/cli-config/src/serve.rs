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
}

impl Default for ServeArguments {
    fn default() -> Self {
        Self {
            port: default_port(),
            addr: None,
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
