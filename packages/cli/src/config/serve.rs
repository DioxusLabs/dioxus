use clap::Parser;

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
