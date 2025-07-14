use crate::{cli::*, AppBuilder, BuildRequest, Workspace};
use crate::{BuildMode, Platform};

use super::target::TargetArgs;

/// Build the Rust Dioxus app and all of its assets.
///
/// Produces a final output build. If a "server" feature is present in the package's Cargo.toml, it will
/// be considered a fullstack app and the server will be built as well.
#[derive(Clone, Debug, Default, Parser)]
pub struct BuildArgs {
    /// Enable fullstack mode [default: false]
    ///
    /// This is automatically detected from `dx serve` if the "fullstack" feature is enabled by default.
    #[clap(long)]
    pub(crate) fullstack: Option<bool>,

    /// Pre-render all routes returned from the app's `/static_routes` endpoint [default: false]
    #[clap(long)]
    pub(crate) ssg: bool,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: TargetArgs,
}

pub struct BuildTargets {
    pub client: BuildRequest,
    pub server: Option<BuildRequest>,
}

impl BuildArgs {
    fn default_client(&self) -> &TargetArgs {
        &self.build_arguments
    }

    fn default_server(&self, client: &BuildRequest) -> Option<&TargetArgs> {
        // Now resolve the builds that we need to.
        // These come from the args, but we'd like them to come from the `TargetCmd` chained object
        //
        // The process here is as follows:
        //
        // - Create the BuildRequest for the primary target
        // - If that BuildRequest is "fullstack", then add the client features
        // - If that BuildRequest is "fullstack", then also create a BuildRequest for the server
        //   with the server features
        //
        // This involves modifying the BuildRequest to add the client features and server features
        // only if we can properly detect that it's a fullstack build. Careful with this, since
        // we didn't build BuildRequest to be generally mutable.
        let default_server = client.enabled_platforms.contains(&Platform::Server);

        // Make sure we set the fullstack platform so we actually build the fullstack variant
        // Users need to enable "fullstack" in their default feature set.
        // todo(jon): fullstack *could* be a feature of the app, but right now we're assuming it's always enabled
        //
        // Now we need to resolve the client features
        let fullstack = ((default_server || client.fullstack_feature_enabled())
            || self.fullstack.unwrap_or(false))
            && self.fullstack != Some(false);

        fullstack.then_some(&self.build_arguments)
    }
}

impl CommandWithPlatformOverrides<BuildArgs> {
    pub async fn build(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let ssg = self.shared.ssg;
        let targets = self.into_targets().await?;

        AppBuilder::started(&targets.client, BuildMode::Base)?
            .finish_build()
            .await?;

        tracing::info!(path = ?targets.client.root_dir(), "Client build completed successfully! 🚀");

        if let Some(server) = targets.server.as_ref() {
            // If the server is present, we need to build it as well
            let mut server_build = AppBuilder::started(server, BuildMode::Base)?;
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

    pub async fn into_targets(self) -> Result<BuildTargets> {
        let workspace = Workspace::current().await?;

        // do some logging to ensure dx matches the dioxus version since we're not always API compatible
        workspace.check_dioxus_version_against_cli();

        let client_args = match &self.client {
            Some(client) => &client.build_arguments,
            None => self.shared.default_client(),
        };
        let client = BuildRequest::new(client_args, None, workspace.clone()).await?;

        let server_args = match &self.server {
            Some(server) => Some(&server.build_arguments),
            None => self.shared.default_server(&client),
        };

        let mut server = None;
        // If there is a server, make sure we output in the same directory as the client build so we use the server
        // to serve the web client
        if let Some(server_args) = server_args {
            // Copy the main target from the client to the server
            let main_target = client.main_target.clone();
            let mut server_args = server_args.clone();
            // The platform in the server build is always set to Server
            server_args.platform = Some(Platform::Server);
            server =
                Some(BuildRequest::new(&server_args, Some(main_target), workspace.clone()).await?);
        }

        Ok(BuildTargets { client, server })
    }
}
