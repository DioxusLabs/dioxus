use dioxus_dx_wire_format::StructuredBuildArtifacts;

use crate::{
    cli::*, Anonymized, AppBuilder, BuildArtifacts, BuildId, BuildMode, BuildRequest, BundleFormat,
    Platform, TargetArgs, Workspace,
};

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

    /// Force a "fat" binary, required to use `dx build-tools hotpatch`
    #[clap(long)]
    pub(crate) fat_binary: bool,

    /// This flag only applies to fullstack builds. By default fullstack builds will run the server
    /// and client builds in parallel. This flag will force the build to run the server build first, then the client build. [default: false]
    ///
    /// If CI is enabled, this will be set to true by default.
    ///
    #[clap(
        long, default_missing_value = "true",
        num_args = 0..=1,
    )]
    pub(crate) force_sequential: Option<bool>,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: TargetArgs,
}

impl BuildArgs {
    pub(crate) fn force_sequential_build(&self) -> bool {
        self.force_sequential
            .unwrap_or_else(|| std::env::var("CI").is_ok())
    }
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
    pub async fn into_targets(mut self) -> Result<BuildTargets> {
        let workspace = Workspace::current().await?;

        // do some logging to ensure dx matches the dioxus version since we're not always API compatible
        workspace.check_dioxus_version_against_cli();

        // The client args are the `@client` arguments, or the shared build arguments if @client is not specified.
        let client_args = &self.client.as_ref().unwrap_or(&self.shared).build_arguments;

        // Create the client build request
        let client = BuildRequest::new(client_args, workspace.clone()).await?;

        // Create the server build request if needed
        let mut server = None;
        if matches!(self.shared.fullstack, Some(true))
            || client.fullstack_feature_enabled()
            || self.server.is_some()
        {
            match self.server.as_mut() {
                Some(server_args) => {
                    // Make sure we set the client target here so @server knows to place its output into the @client target directory.
                    server_args.build_arguments.client_target = Some(client.main_target.clone());

                    // We don't override anything except the bundle format since @server usually implies a server output
                    server_args.build_arguments.bundle = server_args
                        .build_arguments
                        .bundle
                        .or(Some(BundleFormat::Server));

                    server = Some(
                        BuildRequest::new(&server_args.build_arguments, workspace.clone()).await?,
                    );
                }
                None if client_args.platform == Platform::Server => {
                    // If the user requests a server build with `--server`, then we don't need to build a separate server binary.
                    // There's no client to use, so even though fullstack is true, we only build the server.
                }
                None => {
                    let mut args = self.shared.build_arguments.clone();
                    args.platform = crate::Platform::Server;
                    args.renderer = Some(crate::Renderer::Server);
                    args.bundle = Some(crate::BundleFormat::Server);
                    args.target = Some(target_lexicon::Triple::host());
                    server = Some(BuildRequest::new(&args, workspace.clone()).await?);
                }
            }
        }

        Ok(BuildTargets { client, server })
    }

    pub async fn build(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let force_sequential = self.shared.force_sequential_build();
        let ssg = self.shared.ssg;
        let mode = match self.shared.fat_binary {
            true => BuildMode::Fat,
            false => BuildMode::Base { run: false },
        };
        let targets = self.into_targets().await?;

        let build_client = Self::build_client_inner(&targets.client, mode.clone());
        let build_server = Self::build_server_inner(&targets.server, mode.clone(), ssg);

        let (client, server) = match force_sequential {
            true => (build_client.await, build_server.await),
            false => tokio::join!(build_client, build_server),
        };

        Ok(StructuredOutput::BuildsFinished {
            client: client?.into_structured_output(),
            server: server?.map(|s| s.into_structured_output()),
        })
    }

    pub(crate) async fn build_client_inner(
        request: &BuildRequest,
        mode: BuildMode,
    ) -> Result<BuildArtifacts> {
        AppBuilder::started(request, mode, BuildId::PRIMARY)?
            .finish_build()
            .await
            .inspect(|_| {
                tracing::info!(path = ?request.root_dir(), "Client build completed successfully! 🚀");
            })
    }

    pub(crate) async fn build_server_inner(
        request: &Option<BuildRequest>,
        mode: BuildMode,
        ssg: bool,
    ) -> Result<Option<BuildArtifacts>> {
        let Some(server) = request.as_ref() else {
            return Ok(None);
        };

        // If the server is present, we need to build it as well
        let mut server_build = AppBuilder::started(server, mode, BuildId::SECONDARY)?;
        let server_artifacts = server_build.finish_build().await?;

        // Run SSG and cache static routes
        if ssg {
            crate::pre_render_static_routes(None, &mut server_build, None).await?;
        }

        tracing::info!(path = ?server.root_dir(), "Server build completed successfully! 🚀");

        Ok(Some(server_artifacts))
    }
}

impl BuildArtifacts {
    pub(crate) fn into_structured_output(self) -> StructuredBuildArtifacts {
        StructuredBuildArtifacts {
            path: self.root_dir,
            exe: self.exe,
            rustc_args: self.direct_rustc.args,
            rustc_envs: self.direct_rustc.envs,
            link_args: self.direct_rustc.link_args,
            assets: self.assets.unique_assets().cloned().collect(),
        }
    }
}
