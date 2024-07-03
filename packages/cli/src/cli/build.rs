use crate::assets::AssetConfigDropGuard;
use dioxus_cli_config::Platform;

use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
#[clap(name = "build")]
pub struct Build {
    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub release: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run with something in between debug and release mode. This flag will force the build to run in debug mode. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub force_debug: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub force_sequential: bool,

    // Use verbose output [default: false]
    #[clap(long)]
    #[serde(default)]
    pub verbose: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub example: Option<String>,

    /// Build with custom profile
    #[clap(long)]
    pub profile: Option<String>,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub platform: Option<Platform>,

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long)]
    #[serde(default)]
    pub skip_assets: bool,

    /// Space separated list of features to activate
    #[clap(long)]
    pub features: Option<Vec<String>>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long, default_value_t = { "web".to_string() })]
    pub client_feature: String,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long, default_value_t = { "server".to_string() })]
    pub server_feature: String,

    /// Rustc platform triple
    #[clap(long)]
    pub target: Option<String>,

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub cargo_args: Vec<String>,

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true)]
    pub inject_loading_scripts: bool,
}

impl Build {
    /// Note: `rust_flags` argument is only used for the fullstack platform.
    pub fn build(
        self,
        bin: Option<PathBuf>,
        target_dir: Option<&std::path::Path>,
        rust_flags: Option<String>,
    ) -> Result<()> {
        let mut crate_config = dioxus_cli_config::CrateConfig::new(bin)?;
        if let Some(target_dir) = target_dir {
            crate_config.target_dir = target_dir.to_path_buf();
        }

        // change the release state.
        crate_config.with_release(self.release);
        crate_config.with_verbose(self.verbose);

        if self.example.is_some() {
            crate_config.as_example(self.example.clone().unwrap());
        }

        if self.profile.is_some() {
            crate_config.set_profile(self.profile.clone().unwrap());
        }

        if self.features.is_some() {
            crate_config.set_features(self.features.clone().unwrap());
        }

        let platform = self
            .platform
            .unwrap_or(crate_config.dioxus_config.application.default_platform);

        if let Some(target) = self.target.clone() {
            crate_config.set_target(target);
        }

        crate_config.set_cargo_args(self.cargo_args.clone());
        crate_config.extend_with_platform(platform);

        // #[cfg(feature = "plugin")]
        // let _ = crate::plugin::PluginManager::on_build_start(&crate_config, &platform);

        let build_result = match platform {
            Platform::Web => {
                // `rust_flags` are used by fullstack's client build.
                crate::builder::build_web(&crate_config, self.skip_assets, rust_flags)?
            }
            Platform::Desktop => {
                // Since desktop platform doesn't use `rust_flags`, this
                // argument is explicitly set to `None`.
                crate::builder::build_desktop(&crate_config, false, self.skip_assets, None)?
            }
            Platform::Fullstack | Platform::StaticGeneration => {
                // Fullstack mode must be built with web configs on the desktop
                // (server) binary as well as the web binary
                let _config = AssetConfigDropGuard::new();
                todo!()
                // let client_rust_flags = fullstack::client_rust_flags(&self.build);
                // let server_rust_flags = fullstack::server_rust_flags(&self.build);
                // {
                //     let mut web_config = crate_config.clone();
                //     let web_feature = self.build.client_feature;
                //     let features = &mut web_config.features;
                //     match features {
                //         Some(features) => {
                //             features.push(web_feature);
                //         }
                //         None => web_config.features = Some(vec![web_feature]),
                //     };
                //     crate::builder::build_web(
                //         &web_config,
                //         self.build.skip_assets,
                //         Some(client_rust_flags),
                //     )?;
                // }
                // {
                //     let mut desktop_config = crate_config.clone();
                //     let desktop_feature = self.build.server_feature;
                //     let features = &mut desktop_config.features;
                //     match features {
                //         Some(features) => {
                //             features.push(desktop_feature);
                //         }
                //         None => desktop_config.features = Some(vec![desktop_feature]),
                //     };
                //     crate::builder::build_desktop(
                //         &desktop_config,
                //         false,
                //         self.build.skip_assets,
                //         Some(server_rust_flags),
                //     )?
                // }
            }
            _ => unreachable!(),
        };

        let temp = gen_page(&crate_config, build_result.assets.as_ref(), false);

        let mut file = std::fs::File::create(crate_config.out_dir().join("index.html"))?;
        file.write_all(temp.as_bytes())?;

        // #[cfg(feature = "plugin")]
        // let _ = crate::plugin::PluginManager::on_build_finish(&crate_config, &platform);

        Ok(())
    }
}
