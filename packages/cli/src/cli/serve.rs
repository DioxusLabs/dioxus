use super::*;
use crate::settings;
use crate::DioxusCrate;
use crate::{builder::Platform, config::AddressArguments};
use anyhow::Context;
use build::BuildArgs;
use crossterm::tty::IsTty;

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Default, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
#[clap(name = "serve")]
pub(crate) struct ServeArgs {
    /// The arguments for the address the server will run on
    #[clap(flatten)]
    pub(crate) address: AddressArguments,

    /// Open the app in the default browser [default: true - unless cli settings are set]
    #[arg(long, default_missing_value="true", num_args=0..=1)]
    pub(crate) open: Option<bool>,

    /// Enable full hot reloading for the app [default: true - unless cli settings are set]
    #[clap(long, group = "release-incompatible")]
    pub(crate) hot_reload: Option<bool>,

    /// Configure always-on-top for desktop apps [default: true - unless cli settings are set]
    #[clap(long, default_missing_value = "true")]
    pub(crate) always_on_top: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    pub(crate) cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub(crate) args: Vec<String>,

    /// Sets the interval in seconds that the CLI will poll for file changes on WSL.
    #[clap(long, default_missing_value = "2")]
    pub(crate) wsl_file_poll_interval: Option<u16>,

    /// Run the server in interactive mode
    #[arg(long, default_missing_value="true", num_args=0..=1, short = 'i')]
    pub(crate) interactive: Option<bool>,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: BuildArgs,
}

impl ServeArgs {
    /// Start the tui, builder, etc by resolving the arguments and then running the actual top-level serve function
    pub(crate) async fn serve(mut self) -> Result<()> {
        let mut krate = DioxusCrate::new(&self.build_arguments.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.resolve(&mut krate)?;

        crate::serve::serve_all(self, krate).await
    }

    /// Resolve the serve arguments from the arguments or the config
    fn resolve(&mut self, crate_config: &mut DioxusCrate) -> Result<()> {
        // Set config settings.
        let settings = settings::CliSettings::load();

        // Enable hot reload.
        if self.hot_reload.is_none() {
            self.hot_reload = Some(settings.always_hot_reload.unwrap_or(true));
        }

        // Open browser.
        if self.open.is_none() {
            self.open = Some(settings.always_open_browser.unwrap_or_default());
        }

        // Set WSL file poll interval.
        if self.wsl_file_poll_interval.is_none() {
            self.wsl_file_poll_interval = Some(settings.wsl_file_poll_interval.unwrap_or(2));
        }

        // Set always-on-top for desktop.
        if self.always_on_top.is_none() {
            self.always_on_top = Some(settings.always_on_top.unwrap_or(true))
        }

        crate_config.dioxus_config.desktop.always_on_top = self.always_on_top.unwrap_or(true);

        // Resolve the build arguments
        self.build_arguments.resolve(crate_config)?;

        // Since this is a serve, adjust the outdir to be target/dx-dist/<crate name>
        let mut dist_dir = crate_config.out_dir();

        if crate_config.target.is_example() {
            dist_dir = dist_dir.join("examples");
        }

        crate_config.dioxus_config.application.out_dir =
            dist_dir.join(crate_config.executable_name());

        Ok(())
    }

    pub(crate) fn should_hotreload(&self) -> bool {
        self.hot_reload.unwrap_or(true)
    }

    pub(crate) fn build_args(&self) -> BuildArgs {
        self.build_arguments.clone()
    }

    pub(crate) fn interactive_tty(&self) -> bool {
        std::io::stdout().is_tty() && self.interactive.unwrap_or(true)
    }

    pub(crate) fn should_boot_default_server(&self) -> bool {
        match self.build_arguments.platform() {
            Platform::Server => true,
            Platform::Liveview => true,
            Platform::Web | Platform::Desktop | Platform::Ios | Platform::Android => {
                self.build_arguments.fullstack
            }
        }
    }
}

impl std::ops::Deref for ServeArgs {
    type Target = BuildArgs;

    fn deref(&self) -> &Self::Target {
        &self.build_arguments
    }
}
