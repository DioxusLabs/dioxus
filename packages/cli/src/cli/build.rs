use std::sync::Arc;

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

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: TargetArgs,
}

pub struct BuildTargets {
    pub client: BuildRequest,
    pub server: Option<BuildRequest>,
}

impl BuildArgs {
    async fn default_client(&self, workspace: &Arc<Workspace>) -> Result<BuildRequest> {
        let client = BuildRequest::new(&self.build_arguments, workspace.clone()).await?;

        Ok(client)
    }

    async fn default_server(
        &self,
        workspace: &Arc<Workspace>,
        client: &BuildRequest,
    ) -> Result<Option<BuildRequest>> {
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

        if fullstack {
            let mut build_args = self.build_arguments.clone();
            build_args.platform = Some(Platform::Server);

            let _server = BuildRequest::new(&build_args, workspace.clone()).await?;

            Ok(Some(_server))
        } else {
            Ok(None)
        }
    }
}

impl CommandWithPlatformOverrides<BuildArgs> {
    pub async fn build(self) -> Result<StructuredOutput> {
        tracing::info!("Building project...");

        let targets = self.into_targets().await?;

        AppBuilder::start(&targets.client, BuildMode::Base)?
            .finish_build()
            .await?;

        tracing::info!(path = ?targets.client.root_dir(), "Client build completed successfully! 🚀");

        if let Some(server) = targets.server.as_ref() {
            // If the server is present, we need to build it as well
            AppBuilder::start(server, BuildMode::Base)?
                .finish_build()
                .await?;

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

        let client = match self.client {
            Some(client) => BuildRequest::new(&client.build_arguments, workspace.clone()).await?,
            None => self.shared.default_client(&workspace).await?,
        };

        let server = match self.server {
            Some(mut server) => {
                // The server platform is always server
                server.build_arguments.platform = Some(Platform::Server);
                Some(BuildRequest::new(&server.build_arguments, workspace.clone()).await?)
            }
            None => self.shared.default_server(&workspace, &client).await?,
        };

        Ok(BuildTargets { client, server })
    }
}
