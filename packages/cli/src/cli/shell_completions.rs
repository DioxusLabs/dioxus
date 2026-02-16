use super::*;
use crate::{cli, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};

/// Generate shell completions for the specified shell.
#[derive(Clone, Debug, Parser)]
pub(crate) struct ShellCompletions {
    /// The shell to generate completions for.
    #[clap(value_enum)]
    pub shell: Shell,
}

impl ShellCompletions {
    pub fn generate_and_print(self) -> Result<StructuredOutput> {
        let mut cmd = cli::Cli::command();
        generate(
            self.shell,
            &mut cmd,
            env!("CARGO_BIN_NAME"),
            &mut std::io::stdout(),
        );
        Ok(StructuredOutput::Success)
    }
}
