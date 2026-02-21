use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Clone, Debug, Default)]
pub struct Verbosity {
    /// Use verbose output [default: false]
    #[clap(long, global = true)]
    pub(crate) verbose: bool,

    /// Use trace output [default: false]
    #[clap(long, global = true)]
    pub(crate) trace: bool,

    /// Use quiet output - only show warnings, errors, and final status [default: false]
    #[clap(
        long,
        global = true,
        conflicts_with = "verbose",
        conflicts_with = "trace"
    )]
    pub(crate) quiet: bool,

    /// Output logs in JSON format
    #[clap(long, global = true)]
    pub(crate) json_output: bool,

    /// Write *all* logs to a file
    #[clap(long, global = true, help_heading = "Logging Options")]
    pub(crate) log_to_file: Option<PathBuf>,

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
