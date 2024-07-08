use crate::{
    settings::{self, CliSettings},
    DioxusCrate,
};
use build::Build;
use dioxus_cli_config::ServeArguments;
use std::ops::Deref;

use super::*;

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[command(group = clap::ArgGroup::new("release-incompatible").multiple(true).conflicts_with("release"))]
#[clap(name = "serve")]
pub struct Serve {
    /// Arguments for the serve command
    #[clap(flatten)]
    pub(crate) server_arguments: ServeArguments,

    /// Arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) build_arguments: Build,
}

impl Serve {
    /// Resolve the serve arguments from the arguments or the config
    fn resolve(&mut self, crate_config: &mut DioxusCrate) -> Result<()> {
        // Set config settings
        let settings = settings::CliSettings::load();
        if self.server_arguments.hot_reload.is_none() {
            self.server_arguments.hot_reload = Some(settings.always_hot_reload.unwrap_or(true));
        }
        if self.server_arguments.open.is_none() {
            self.server_arguments.open = Some(settings.always_open_browser.unwrap_or_default());
        }

        // Resolve the build arguments
        self.build_arguments.resolve(crate_config)?;

        Ok(())
    }

    pub fn serve(mut self, mut dioxus_crate: DioxusCrate) -> Result<()> {
        self.resolve(&mut dioxus_crate)?;

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(crate::server::serve_all(self, dioxus_crate))
    }
}

impl Deref for Serve {
    type Target = Build;

    fn deref(&self) -> &Self::Target {
        &self.build_arguments
    }
}
