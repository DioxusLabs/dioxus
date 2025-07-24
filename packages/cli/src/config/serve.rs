use clap::Parser;
use serde_json::{json, Value};

use crate::telemetry::Anonymized;

/// The arguments for the address the server will run on
#[derive(Clone, Debug, Default, Parser)]
pub(crate) struct AddressArguments {
    /// The port the server will run on
    #[clap(long)]
    pub(crate) port: Option<u16>,

    /// The address the server will run on
    #[clap(long)]
    pub(crate) addr: Option<std::net::IpAddr>,
}

impl Anonymized for AddressArguments {
    fn anonymized(&self) -> Value {
        json!({
            "port": self.port,
            "addr": self.addr.map(|addr| if addr.is_loopback() { "loopback" } else if addr.is_unspecified() { "unspecified" } else { "other" }),
        })
    }
}
