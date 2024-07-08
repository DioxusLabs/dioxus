use dioxus_cli_config::Platform;

use crate::{builder::BuildRequest, dioxus_crate::DioxusCrate};

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
    pub features: Vec<String>,

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
    pub fn resolve(&mut self, dioxus_crate: &mut DioxusCrate) -> Result<()> {
        // Forward the build arguments to the resolved config
        if self.example.is_some() {
            dioxus_crate.as_example(self.example.clone().unwrap());
        }

        // Inherit the platform from the defaults
        let platform = self
            .platform
            .unwrap_or_else(|| self.auto_detect_platform(dioxus_crate));
        self.platform = Some(platform);

        // Add any features required to turn on the platform we are building for
        self.features
            .extend(dioxus_crate.features_for_platform(platform));

        Ok(())
    }

    pub async fn build(mut self, mut dioxus_crate: DioxusCrate) -> Result<()> {
        self.resolve(&mut dioxus_crate)?;
        let build_requests = BuildRequest::create(false, dioxus_crate, self);
        let mut tasks = tokio::task::JoinSet::new();
        for build_request in build_requests {
            tasks.spawn(async move { build_request.build().await });
        }

        while let Some(result) = tasks.join_next().await {
            result.map_err(|err| {
                crate::Error::Unique("Panic while building project".to_string())
            })??;
        }
        Ok(())
    }

    fn auto_detect_platform(&self, resolved: &DioxusCrate) -> Platform {
        use cargo_toml::Dependency::{Detailed, Inherited, Simple};

        if let Some(dependency) = &resolved.manifest.dependencies.get("dioxus") {
            let features = match dependency {
                Inherited(detail) => detail.features.to_vec(),
                Detailed(detail) => detail.features.to_vec(),
                Simple(_) => vec![],
            };

            if let Some(platform) = features
                .iter()
                .find_map(|platform| platform.parse::<Platform>().ok())
            {
                return platform;
            }
        }

        resolved.dioxus_config.application.default_platform
    }
}
