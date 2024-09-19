#![allow(unused)] // lots of configs...

use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

/// The arguments for the address the server will run on
#[derive(Clone, Debug, Parser)]
pub(crate) struct AddressArguments {
    /// The port the server will run on
    #[clap(long)]
    #[clap(default_value_t = default_port())]
    pub(crate) port: u16,

    /// The address the server will run on
    #[clap(long, default_value_t = default_address())]
    pub(crate) addr: std::net::IpAddr,
}

impl Default for AddressArguments {
    fn default() -> Self {
        Self {
            port: default_port(),
            addr: default_address(),
        }
    }
}

fn default_port() -> u16 {
    8080
}

fn default_address() -> IpAddr {
    IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
}
