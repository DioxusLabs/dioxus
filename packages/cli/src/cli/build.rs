use crate::{cli::*, AppBuilder, BuildRequest, Workspace, PROFILE_SERVER};
use crate::{BuildMode, Platform};
use target_lexicon::Triple;

use super::target::{TargetArgs, TargetCmd};

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

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long)]
    pub(crate) client_features: Vec<String>,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long)]
    pub(crate) server_features: Vec<String>,

    /// Build with custom profile for the fullstack server
    #[clap(long, default_value_t = PROFILE_SERVER.to_string())]
    pub(crate) server_profile: String,

    /// The target to build for the server.
    ///
    /// This can be different than the host allowing cross-compilation of the server. This is useful for
    /// platforms like Cloudflare Workers where the server is compiled to wasm and then uploaded to the edge.
    #[clap(long)]
    pub(crate) server_target: Option<Triple>,

    /// Arguments for the build itself
    #[clap(flatten)]
    pub(crate) build_arguments: TargetArgs,

    /// A list of additional targets to build.
    ///
    /// Server and Client are special targets that integrate with `dx serve`, while `crate` is a generic.
    ///
    /// ```sh
    /// dx serve \
    ///     client --target aarch64-apple-darwin \
    ///     server --target wasm32-unknown-unknown \
    ///     crate --target aarch64-unknown-linux-gnu --package foo \
    ///     crate --target x86_64-unknown-linux-gnu --package bar
    /// ```
    #[command(subcommand)]
    pub(crate) targets: Option<TargetCmd>,
}

pub struct BuildTargets {
    pub client: BuildRequest,
    pub server: Option<BuildRequest>,
}

impl BuildArgs {
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

        let mut server = None;

        let client = match self.targets {
            // A simple `dx serve` command with no explicit targets
            None => {
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
                let client = BuildRequest::new(&self.build_arguments, workspace.clone()).await?;
                let default_server = client
                    .enabled_platforms
                    .iter()
                    .any(|p| *p == Platform::Server);

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

                    // ... todo: add the server features to the server build
                    // ... todo: add the client features to the client build
                    // // Make sure we have a server feature if we're building a fullstack app
                    if self.fullstack.unwrap_or_default() && self.server_features.is_empty() {
                        return Err(anyhow::anyhow!("Fullstack builds require a server feature on the target crate. Add a `server` feature to the crate and try again.").into());
                    }

                    server = Some(_server);
                }

                client
            }

            // A command in the form of:
            // ```
            // dx serve \
            //     client --package frontend \
            //     server --package backend
            // ```
            Some(cmd) => {
                let mut client_args_ = None;
                let mut server_args_ = None;
                let mut cmd_outer = Some(Box::new(cmd));
                while let Some(cmd) = cmd_outer.take() {
                    match *cmd {
                        TargetCmd::Client(cmd_) => {
                            client_args_ = Some(cmd_.inner);
                            cmd_outer = cmd_.next;
                        }
                        TargetCmd::Server(cmd) => {
                            server_args_ = Some(cmd.inner);
                            cmd_outer = cmd.next;
                        }
                    }
                }

                if let Some(server_args) = server_args_ {
                    server = Some(BuildRequest::new(&server_args, workspace.clone()).await?);
                }

                BuildRequest::new(&client_args_.unwrap(), workspace.clone()).await?
            }
        };

        Ok(BuildTargets { client, server })
    }
}
