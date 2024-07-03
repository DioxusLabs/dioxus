use build::Build;
use dioxus_cli_config::ServeArguments;
use dioxus_cli_config::{CrateConfig, ExecutableType, Platform};
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

    // TODO: Somehow make this default to `true` if the flag was provided. e.g. `dx serve --open`
    // Currently it requires a value: `dx serve --open true`
    /// Open the app in the default browser [default: false - unless project or global settings are set]
    #[clap(long)]
    pub open: Option<bool>,

    // TODO: See `open` field
    /// Enable full hot reloading for the app [default: true - unless project or global settings are set]
    #[clap(long, group = "release-incompatible")]
    pub hot_reload: Option<bool>,

    /// Set cross-origin-policy to same-origin [default: false]
    #[clap(name = "cross-origin-policy")]
    #[clap(long)]
    #[serde(default)]
    pub cross_origin_policy: bool,

    /// Additional arguments to pass to the executable
    #[clap(long)]
    pub args: Vec<String>,
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

        if self.hot_reload.is_none() {
            let value = cli_settings.always_hot_reload.unwrap_or(true);
            self.hot_reload = Some(value);
            crate_config.with_hot_reload(value);
        }

        if self.open.is_none() {
            self.open = Some(cli_settings.always_open_browser.unwrap_or(false));
        }

        // Set config settings
        crate_config.with_cross_origin_policy(self.cross_origin_policy);
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
        crate_config.set_platform_auto_detect(self.platform.clone());

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(crate::server::serve_all(self, crate_config))
    }
}
