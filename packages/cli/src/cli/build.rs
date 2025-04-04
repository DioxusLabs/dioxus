use super::*;
use crate::{Builder, DioxusCrate, Platform, PROFILE_SERVER};

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output bundle designed to be run on the target platform.
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct BuildArgs {
    /// Build in release mode [default: false]
    #[clap(long, short)]
    #[serde(default)]
    pub(crate) release: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) force_sequential: bool,

    /// Build the app with custom a profile
    #[clap(long)]
    pub(crate) profile: Option<String>,

    /// Build with custom profile for the fullstack server
    #[clap(long, default_value_t = PROFILE_SERVER.to_string())]
    pub(crate) server_profile: String,

    /// Build platform: support Web & Desktop [default: "default_platform"]
    #[clap(long, value_enum)]
    pub(crate) platform: Option<Platform>,

    /// Build the fullstack variant of this app, using that as the fileserver and backend
    ///
    /// This defaults to `false` but will be overridden to true if the `fullstack` feature is enabled.
    #[arg(long, default_missing_value="true", num_args=0..=1)]
    pub(crate) fullstack: Option<bool>,

    /// Run the ssg config of the app and generate the files
    #[clap(long)]
    pub(crate) ssg: bool,

    /// Skip collecting assets from dependencies [default: false]
    #[clap(long)]
    #[serde(default)]
    pub(crate) skip_assets: bool,

    /// Extra arguments passed to cargo build
    #[clap(last = true)]
    pub(crate) cargo_args: Vec<String>,

    /// Inject scripts to load the wasm and js files for your dioxus app if they are not already present [default: true]
    #[clap(long, default_value_t = true)]
    pub(crate) inject_loading_scripts: bool,

    /// Experimental: Bundle split the wasm binary into multiple chunks based on `#[wasm_split]` annotations [default: false]
    #[clap(long, default_value_t = false)]
    pub(crate) experimental_wasm_split: bool,

    /// Generate debug symbols for the wasm binary [default: true]
    ///
    /// This will make the binary larger and take longer to compile, but will allow you to debug the
    /// wasm binary
    #[clap(long, default_value_t = true)]
    pub(crate) debug_symbols: bool,

    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) target_args: TargetArgs,
}

impl BuildArgs {
    pub async fn run_cmd(mut self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let krate =
            DioxusCrate::new(&self.target_args).context("Failed to load Dioxus workspace")?;

        self.resolve(&krate).await?;

        let bundle = Builder::start(&krate, self.clone())?.finish().await?;

        tracing::info!(path = ?bundle.build.root_dir(), "Build completed successfully! 🚀");

        Ok(StructuredOutput::BuildFinished {
            path: bundle.build.root_dir(),
        })
    }

    /// Update the arguments of the CLI by inspecting the DioxusCrate itself and learning about how
    /// the user has configured their app.
    ///
    /// IE if they've specified "fullstack" as a feature on `dioxus`, then we want to build the
    /// fullstack variant even if they omitted the `--fullstack` flag.
    pub(crate) async fn resolve(&mut self, krate: &DioxusCrate) -> Result<()> {
        let default_platforms = krate.default_platforms();
        let default_platform = default_platforms.iter().find(|p| **p != Platform::Server);
        let default_server = default_platforms.iter().any(|p| *p == Platform::Server);
        let auto_platform = krate.autodetect_platform();

        // Make sure we set the fullstack platform so we actually build the fullstack variant
        // Users need to enable "fullstack" in their default feature set or explicitly pass the flag
        self.fullstack = Some(
            self.fullstack()
                || self.fullstack.is_none()
                    && (default_server || krate.has_dioxus_feature("fullstack")),
        );

        // If the current build is a fullstack build which includes either the client or the server in the default features,
        // remove that default feature and just add it back into the client or server args. If they passed in an explicit platform
        // but they also have a default feature platform, strip out the default features and add back in the platform they passed in.
        if self.fullstack() && (default_server || default_platform.is_some())
            || self.platform.is_some() && default_platform.is_some()
        {
            self.target_args.no_default_features = true;
            self.target_args
                .features
                .extend(krate.platformless_features());
        }

        // Inherit the platform from the args, or auto-detect it
        if self.platform.is_none() {
            let (platform, _feature) = auto_platform.ok_or_else(|| {
                anyhow::anyhow!("No platform was specified and could not be auto-detected. Please specify a platform with `--platform <platform>` or set a default platform using a cargo feature.")
            })?;
            self.platform = Some(platform);
        }

        let platform = self
            .platform
            .expect("Platform to be set after autodetection");

        // Add any features required to turn on the client
        self.target_args
            .client_features
            .push(krate.feature_for_platform(platform));

        // Add any features required to turn on the server
        // This won't take effect in the server is not built, so it's fine to just set it here even if it's not used
        self.target_args
            .server_features
            .push(krate.feature_for_platform(Platform::Server));

        // Make sure we have a server feature if we're building a fullstack app
        //
        // todo(jon): eventually we want to let users pass a `--server <crate>` flag to specify a package to use as the server
        // however, it'll take some time to support that and we don't have a great RPC binding layer between the two yet
        if self.fullstack() && self.target_args.server_features.is_empty() {
            return Err(anyhow::anyhow!("Fullstack builds require a server feature on the target crate. Add a `server` feature to the crate and try again.").into());
        }

        // Set the profile of the build if it's not already set
        // We do this for android/wasm since they require
        if self.profile.is_none() && !self.release {
            match self.platform {
                Some(Platform::Android) => {
                    self.profile = Some(crate::dioxus_crate::PROFILE_ANDROID.to_string());
                }
                Some(Platform::Web) => {
                    self.profile = Some(crate::dioxus_crate::PROFILE_WASM.to_string());
                }
                Some(Platform::Server) => {
                    self.profile = Some(crate::dioxus_crate::PROFILE_SERVER.to_string());
                }
                _ => {}
            }
        }

        // Determine arch if android
        if self.platform == Some(Platform::Android) && self.target_args.arch.is_none() {
            tracing::debug!("No android arch provided, attempting to auto detect.");

            let arch = DioxusCrate::autodetect_android_arch().await;

            // Some extra logs
            let arch = match arch {
                Some(a) => {
                    tracing::debug!(
                        "Autodetected `{}` Android arch.",
                        a.android_target_triplet()
                    );
                    a.to_owned()
                }
                None => {
                    let a = Arch::default();
                    tracing::debug!(
                        "Could not detect Android arch, defaulting to `{}`",
                        a.android_target_triplet()
                    );
                    a
                }
            };

            self.target_args.arch = Some(arch);
        }

        Ok(())
    }

    /// Get the platform from the build arguments
    pub(crate) fn platform(&self) -> Platform {
        self.platform.expect("Platform was not set")
    }

    /// Check if this is a fullstack build
    pub(crate) fn fullstack(&self) -> bool {
        self.fullstack.unwrap_or(false)
    }
}
