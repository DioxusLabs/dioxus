use crate::{cli::*, Anonymized, AppBuilder, BuildMode, BuildRequest, TargetArgs, Workspace};

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output build. If a "server" feature is present in the package's Cargo.toml, it will
/// be considered a fullstack app and the server will be built as well.
#[derive(Clone, Debug, Default, Parser)]
pub struct BuildArgs {
    /// Enable fullstack mode [default: false]
    ///
    /// This is automatically detected from `dx serve` if the "fullstack" feature is enabled by default.
    #[arg(
        long,
        default_missing_value = "true",
        num_args = 0..=1,
    )]
    pub(crate) fullstack: Option<bool>,

    /// Pre-render all routes returned from the app's `/static_routes` endpoint [default: false]
    #[clap(long)]
    pub(crate) ssg: bool,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: TargetArgs,
}

impl Anonymized for BuildArgs {
    fn anonymized(&self) -> Value {
        json! {{
            "fullstack": self.fullstack,
            "ssg": self.ssg,
            "build_arguments": self.build_arguments.anonymized(),
        }}
    }
}

pub struct BuildTargets {
    pub client: BuildRequest,
    pub server: Option<BuildRequest>,
}

impl CommandWithPlatformOverrides<BuildArgs> {
    /// We need to decompose the combined `BuildArgs` into the individual targets that we need to build.
    ///
    /// Only in a few cases do we spin out an additional server binary:
    /// - the fullstack feature is passed
    /// - the fullstack flag is enabled
    /// - the server flag is enabled
    ///
    /// The buildtargets configuration comes in two flavors:
    /// - implied via the `fullstack` feature
    /// - explicit when using `@server and @client`
    ///
    /// We use the client arguments to build the client target, and then make a few changes to make
    /// the server target.
    ///
    /// The `--fullstack` feature is basically the same as passing `--features fullstack`
    ///
    /// Some examples:
    /// ```shell, ignore
    /// dx serve --target wasm32-unknown-unknown --fullstack            # serves both client and server
    /// dx serve --target wasm32-unknown-unknown --features fullstack   # serves both client and server
    /// dx serve --target wasm32-unknown-unknown                        # only serves the client
    /// dx serve --target wasm32-unknown-unknown                        # servers both if `fullstack` is enabled on dioxus
    /// dx serve @client --target wasm32-unknown-unknown                # only serves the client
    /// dx serve @client --target wasm32-unknown-unknown --fullstack    # serves both client and server
    /// ```
    ///
    /// Currently it is not possible to serve the server without the client, but this could be added in the future.
    pub async fn into_targets(self) -> Result<BuildTargets> {
        let workspace = Workspace::current().await?;

        // do some logging to ensure dx matches the dioxus version since we're not always API compatible
        workspace.check_dioxus_version_against_cli();

        // The client args are the `@client` arguments, or the shared build arguments if @client is not specified.
        let client_args = &self.client.as_ref().unwrap_or(&self.shared).build_arguments;

        // Create the client build request
        let client = BuildRequest::new(client_args, None, workspace.clone()).await?;

        // Create the server build request if needed
        // This happens when 1) fullstack is enabled, 2)
        let mut server = None;
        if matches!(self.shared.fullstack, Some(true))
            || client.fullstack_feature_enabled()
            || self.server.is_some()
        {
            match self.server.as_ref() {
                Some(server_args) => {
                    server = Some(
                        BuildRequest::new(
                            &server_args.build_arguments,
                            Some(client.main_target.clone()),
                            workspace.clone(),
                        )
                        .await?,
                    );
                }
                None => {
                    let mut args = self.shared.build_arguments.clone();
                    args.platform = Some(crate::Platform::Server);
                    args.renderer.renderer = Some(crate::Renderer::Server);
                    args.target = Some(target_lexicon::Triple::host());
                    server = Some(BuildRequest::new(&args, None, workspace.clone()).await?);
                }
            }
        }

        Ok(BuildTargets { client, server })
    }

    pub async fn build(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let ssg = self.shared.ssg;
        let targets = self.into_targets().await?;

        AppBuilder::started(&targets.client, BuildMode::Base { run: false })?
            .finish_build()
            .await?;

        tracing::info!(path = ?targets.client.root_dir(), "Client build completed successfully! 🚀");

        if let Some(server) = targets.server.as_ref() {
            // If the server is present, we need to build it as well
            let mut server_build = AppBuilder::started(server, BuildMode::Base { run: false })?;
            server_build.finish_build().await?;

            // Run SSG and cache static routes
            if ssg {
                crate::pre_render_static_routes(None, &mut server_build, None).await?;
            }

            tracing::info!(path = ?targets.client.root_dir(), "Server build completed successfully! 🚀");
        }

        Ok(StructuredOutput::BuildsFinished {
            client: targets.client.root_dir(),
            server: targets.server.map(|s| s.root_dir()),
        })
    }
}
