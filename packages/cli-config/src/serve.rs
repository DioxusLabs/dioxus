#![allow(unused)] // lots of configs...

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

#[cfg(feature = "read-from-args")]
use clap::Parser;

/// The arguments for the address the server will run on

#[cfg(feature = "read-from-args")]
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

#[cfg(feature = "read-from-args")]
impl Default for AddressArguments {
    fn default() -> Self {
        Self {
            port: default_port(),
            addr: default_address(),
        }
    }
}

#[cfg(feature = "read-from-args")]
impl AddressArguments {
    /// Get the address the server should run on
    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.addr, self.port)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RuntimeCLIArguments {
    /// The address hot reloading is running on
    cli_address: SocketAddr,

    /// The address the server should run on
    server_socket: Option<SocketAddr>,
}

impl RuntimeCLIArguments {
    /// Create a new RuntimeCLIArguments
    pub fn new(cli_address: SocketAddr, server_socket: Option<SocketAddr>) -> Self {
        Self {
            cli_address,
            server_socket,
        }
    }

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

    /// Get the address the CLI is running on
    pub fn cli_address(&self) -> SocketAddr {
        self.cli_address
    }

    /// Get the address the proxied fullstack server should run on
    #[cfg(feature = "read-from-args")]
    pub fn fullstack_address(&self) -> AddressArguments {
        let socket = self.server_socket.unwrap_or_else(|| {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, default_port()))
        });

        AddressArguments {
            port: socket.port(),
            addr: socket.ip(),
        }
    }
}

#[cfg(feature = "read-from-args")]
fn default_port() -> u16 {
    8080
}

#[cfg(feature = "read-from-args")]
fn default_address() -> IpAddr {
    IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
}
