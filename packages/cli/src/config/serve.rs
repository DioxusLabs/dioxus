#![allow(unused)] // lots of configs...

use clap::Parser;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

/// The arguments for the address the server will run on
#[derive(Clone, Debug, Parser)]
pub(crate) struct AddressArguments {
    /// The port the server will run on
    #[clap(long)]
    pub(crate) port: Option<u16>,

    /// The address the server will run on
    #[clap(long)]
    pub(crate) addr: Option<std::net::IpAddr>,
}

impl Default for AddressArguments {
    fn default() -> Self {
        Self {
            port: None,
            addr: None,
        }
    }
}
