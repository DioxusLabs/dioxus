use super::*;

/// Run the WASM project on dev-server
#[derive(Clone, Debug, Parser)]
#[clap(name = "serve")]
pub struct Serve {
    #[clap(flatten)]
    pub serve: ConfigOptsServe,
}

impl Serve {
    pub fn serve(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;
        let mut serve_cfg = self.serve.clone();

        // Handle cli settings
        let cli_settings = crate_config.dioxus_config.cli_settings.clone().unwrap();

        if serve_cfg.hot_reload.is_none() {
            let value = cli_settings.always_hot_reload.unwrap_or(true);
            serve_cfg.hot_reload = Some(value);
            crate_config.with_hot_reload(value);
        }

        if serve_cfg.open.is_none() {
            serve_cfg.open = Some(cli_settings.always_open_browser.unwrap_or(false));
        }

        // Set config settings
        crate_config.with_cross_origin_policy(self.serve.cross_origin_policy);
        crate_config.with_release(self.serve.release);
        crate_config.with_verbose(self.serve.verbose);

        if let Some(example) = self.serve.example {
            crate_config.as_example(example);
        }

        if let Some(profile) = self.serve.profile {
            crate_config.set_profile(profile);
        }

        if let Some(features) = self.serve.features {
            crate_config.set_features(features);
        }

        if let Some(target) = self.serve.target {
            crate_config.set_target(target);
        }

        crate_config.set_cargo_args(self.serve.cargo_args);
        crate_config.set_platform_auto_detect(self.serve.platform);

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(crate::server::serve_all(serve_cfg, crate_config))
    }
}
