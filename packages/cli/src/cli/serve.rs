use crate::config::AddressArguments;
use crate::settings;
use crate::DioxusCrate;
use anyhow::Context;
use build::BuildArgs;
use std::ops::Deref;

use super::*;

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Default, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
#[clap(name = "serve")]
pub struct ServeArgs {
    /// The arguments for the address the server will run on
    #[clap(flatten)]
    pub address: AddressArguments,

    /// Open the app in the default browser [default: true - unless cli settings are set]
    #[arg(long, default_missing_value="true", num_args=0..=1)]
    pub open: Option<bool>,

    /// Enable full hot reloading for the app [default: true - unless cli settings are set]
    #[clap(long, group = "release-incompatible")]
    pub hot_reload: Option<bool>,

    /// Configure always-on-top for desktop apps [default: true - unless cli settings are set]
    #[clap(long, default_missing_value = "true")]
    pub always_on_top: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    pub cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub args: Vec<String>,

    /// Sets the interval in seconds that the CLI will poll for file changes on WSL.
    #[clap(long, default_missing_value = "2")]
    pub wsl_file_poll_interval: Option<u16>,

    /// Arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) build_arguments: BuildArgs,

    /// Run the server in interactive mode
    #[arg(long, default_missing_value="true", num_args=0..=1, short = 'i')]
    pub interactive: Option<bool>,
}

impl ServeArgs {
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
        let mut dist_dir = crate_config.workspace_dir().join("target").join("dx-dist");

        if crate_config.target.is_example() {
            dist_dir = dist_dir.join("examples");
        }

        crate_config.dioxus_config.application.out_dir =
            dist_dir.join(crate_config.executable_name());

        Ok(())
    }

    pub async fn serve(mut self) -> anyhow::Result<()> {
        let mut dioxus_crate = DioxusCrate::new(&self.build_arguments.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.resolve(&mut dioxus_crate)?;

        crate::serve::serve_all(self, dioxus_crate).await?;
        Ok(())
    }

    pub fn should_hotreload(&self) -> bool {
        self.hot_reload.unwrap_or(true)
    }
}

impl Deref for ServeArgs {
    type Target = BuildArgs;

    fn deref(&self) -> &Self::Target {
        &self.build_arguments
    }
}
