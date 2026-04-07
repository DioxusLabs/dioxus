use super::*;
use crate::{cli, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::sync::atomic::AtomicBool;

/// Generate shell completions for the specified shell.
#[derive(Clone, Debug, Parser)]
pub(crate) struct ShellCompletions {
    /// The shell to generate completions for.
    #[clap(value_enum)]
    pub shell: Shell,
}

/// An annoying, code smelly, way of setting the CLI recursive depth parser to limit early, useful for
/// generating completions here.
///
/// This should be set before calling clap_complete's generation function
pub static GENERATING_COMPLETIONS: AtomicBool = AtomicBool::new(false);

impl ShellCompletions {
    pub fn generate_and_print(self) -> Result<StructuredOutput> {
        let mut cmd = cli::Cli::command();
        GENERATING_COMPLETIONS.swap(true, std::sync::atomic::Ordering::Relaxed);
        generate(
            self.shell,
            &mut cmd,
            env!("CARGO_BIN_NAME"),
            &mut std::io::stdout(),
        );
        Ok(StructuredOutput::Success)
    }
}
