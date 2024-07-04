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

impl Deref for Serve {
    type Target = Build;

    fn deref(&self) -> &Self::Target {
        &self.build_arguments
    }
}

impl Serve {
    pub fn serve(mut self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;

        // Handle cli settings
        let cli_settings = crate_config.dioxus_config.cli_settings.clone().unwrap();

        if self.server_arguments.hot_reload.is_none() {
            self.server_arguments.hot_reload = Some(cli_settings.always_hot_reload.unwrap_or(true));
        }

        self.server_arguments.open |= cli_settings.always_open_browser.unwrap_or_default();

        // Set config settings
        crate_config.with_cross_origin_policy(self.server_arguments.cross_origin_policy);
        crate_config.with_release(self.release);
        crate_config.with_verbose(self.verbose);

        if let Some(example) = self.example.clone() {
            crate_config.as_example(example);
        }

        if let Some(profile) = self.profile.clone() {
            crate_config.set_profile(profile);
        }

        if let Some(features) = self.features.clone() {
            crate_config.set_features(features);
        }

        if let Some(target) = self.target.clone() {
            crate_config.set_target(target);
        }

        crate_config.set_cargo_args(self.cargo_args.clone());
        crate_config.set_platform_auto_detect(self.platform);

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(crate::server::serve_all(self, crate_config))
    }
}
