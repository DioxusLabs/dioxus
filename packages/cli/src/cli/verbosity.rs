use clap::Parser;

#[derive(Parser, Clone, Copy, Debug, Default)]
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

    /// Assert that `Cargo.lock` will remain unchanged
    #[clap(long, global = true, help_heading = "Manifest Options")]
    pub(crate) locked: bool,

    /// Run without accessing the network
    #[clap(long, global = true, help_heading = "Manifest Options")]
    pub(crate) offline: bool,

    /// Equivalent to specifying both --locked and --offline
    #[clap(long, global = true, help_heading = "Manifest Options")]
    pub(crate) frozen: bool,
}
