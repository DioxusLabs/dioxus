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
    pub fn resolve(&mut self, crate_config: &mut DioxusCrate) -> Result<()> {
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
}

impl Deref for Serve {
    type Target = Build;

    fn deref(&self) -> &Self::Target {
        &self.build_arguments
    }
}

impl Serve {
    pub fn serve(mut self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = crate::dioxus_crate::DioxusCrate::new(bin)?;

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(crate::server::serve_all(self, crate_config))
    }
}
