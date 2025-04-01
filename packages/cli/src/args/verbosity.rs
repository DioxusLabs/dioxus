use clap::Parser;

#[derive(Parser, Clone, Copy, Debug)]
pub struct Verbosity {
    /// Use verbose output [default: false]
    #[clap(long, global = true)]
    pub(crate) verbose: bool,

    /// Use trace output [default: false]
    #[clap(long, global = true)]
    pub(crate) trace: bool,

    /// Output logs in JSON format
    #[clap(long, global = true)]
    pub(crate) json_output: bool,
}
