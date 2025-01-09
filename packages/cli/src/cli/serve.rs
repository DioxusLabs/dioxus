use super::*;
use crate::{AddressArguments, BuildArgs, CliSettings, DioxusCrate, Platform};

/// Serve the project
#[derive(Clone, Debug, Default, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
pub(crate) struct ServeArgs {
    /// The arguments for the address the server will run on
    #[clap(flatten)]
    pub(crate) address: AddressArguments,

    /// Open the app in the default browser [default: true - unless cli settings are set]
    #[arg(long, num_args=0..=1)]
    open: Option<bool>,

    /// Enable full hot reloading for the app [default: true - unless cli settings are set]
    #[clap(long, group = "release-incompatible")]
    hot_reload: Option<bool>,

    /// Configure always-on-top for desktop apps [default: true - unless cli settings are set]
    #[clap(long)]
    always_on_top: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    pub(crate) cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub(crate) args: Vec<String>,

    /// Sets the interval in seconds that the CLI will poll for file changes on WSL.
    #[clap(long)]
    wsl_file_poll_interval: Option<u16>,

    /// Run the server in interactive mode
    #[arg(long, default_missing_value="true", num_args=0..=1, short = 'i')]
    pub(crate) interactive: Option<bool>,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: BuildArgs,
}

impl ServeArgs {
    /// Start the tui, builder, etc by resolving the arguments and then running the actual top-level serve function
    ///
    /// Make sure not to do any intermediate logging since our tracing infra has now enabled much
    /// higher log levels
    pub(crate) async fn serve(self) -> Result<StructuredOutput> {
        crate::serve::serve_all(self).await?;

        Ok(StructuredOutput::Success)
    }

    pub(crate) async fn load_krate(&mut self) -> Result<DioxusCrate> {
        let override_settings = CliSettings {
            always_hot_reload: self.hot_reload,
            always_open_browser: self.open,
            always_on_top: self.always_on_top,
            wsl_file_poll_interval: self.wsl_file_poll_interval,
        };

        let krate = DioxusCrate::new(&self.build_arguments.target_args, Some(override_settings))?;
        self.resolve(&krate).await?;
        Ok(krate)
    }

    pub(crate) async fn resolve(&mut self, krate: &DioxusCrate) -> Result<()> {
        self.build_arguments.resolve(krate).await?;
        Ok(())
    }

    pub(crate) fn build_args(&self) -> BuildArgs {
        self.build_arguments.clone()
    }

    pub(crate) fn is_interactive_tty(&self) -> bool {
        use crossterm::tty::IsTty;
        std::io::stdout().is_tty() && self.interactive.unwrap_or(true)
    }

    pub(crate) fn should_proxy_build(&self) -> bool {
        match self.build_arguments.platform() {
            Platform::Server => true,
            _ => self.build_arguments.fullstack,
        }
    }
}

impl std::ops::Deref for ServeArgs {
    type Target = BuildArgs;

    fn deref(&self) -> &Self::Target {
        &self.build_arguments
    }
}
