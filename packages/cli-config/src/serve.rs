use std::net::{IpAddr, SocketAddr};

use clap::Parser;

/// The arguments for the address the server will run on
#[derive(Clone, Debug, Parser)]
pub struct AddressArguments {
    /// The port the server will run on
    #[clap(long)]
    #[clap(default_value_t = default_port())]
    pub port: u16,

    /// The address the server will run on
    #[clap(long, default_value_t = default_address())]
    pub addr: std::net::IpAddr,
}

impl Default for AddressArguments {
    fn default() -> Self {
        Self {
            port: default_port(),
            addr: default_address(),
        }
    }
}

impl AddressArguments {
    /// Get the address the server should run on
    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.addr, self.port)
    }
}

/// Arguments for the serve command
#[derive(Clone, Debug, Parser, Default)]
pub struct ServeArguments {
    /// The arguments for the address the server will run on
    #[clap(flatten)]
    pub address: AddressArguments,

    /// Open the app in the default browser [default: false - unless project or global settings are set]
    #[clap(long)]
    pub open: Option<bool>,

    /// Enable full hot reloading for the app [default: true - unless project or global settings are set]
    #[clap(long, group = "release-incompatible")]
    pub hot_reload: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    pub cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RuntimeCLIArguments {
    /// The address hot reloading is running on
    pub cli_address: SocketAddr,

    /// The address the server should run on
    pub server_socket: Option<SocketAddr>,
}

impl RuntimeCLIArguments {
    /// Attempt to read the current serve settings from the CLI. This will only be set for the fullstack platform on recent versions of the CLI.
    pub fn from_cli() -> Option<Self> {
        std::env::var(crate::__private::SERVE_ENV)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
    }

    /// Get the address the server should run on
    pub fn server_socket(&self) -> Option<SocketAddr> {
        self.server_socket
    }
}

impl From<RuntimeCLIArguments> for AddressArguments {
    fn from(args: RuntimeCLIArguments) -> Self {
        Self {
            port: args.cli_address.port(),
            addr: args.cli_address.ip(),
        }
    }
}

fn default_port() -> u16 {
    8080
}

fn default_address() -> IpAddr {
    IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))
}
