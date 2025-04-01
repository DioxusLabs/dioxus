use std::collections::HashMap;

use crate::*;

use super::ServeUpdate;

/// This handles ongoing builds, bundles, and handles to running apps
///
/// Previously we separate these concepts, however with patching integration, we need to store significantly
/// more ongoing state about apps and their builds.
pub struct Serve {
    pub(crate) args: ServeArgs,

    pub(crate) builds: Vec<BuildRequest>,

    pub(crate) interactive: bool,

    pub(crate) force_sequential: bool,

    pub(crate) hotreload: bool,

    pub(crate) open_browser: bool,

    pub(crate) wsl_file_poll_interval: bool,

    pub(crate) always_on_top: bool,
}

impl Serve {
    pub async fn new(args: ServeArgs) -> Result<Self> {
        // let mut builder = Builder::start(&state)?;

        // todo: verify the tooling...?
        // maybe do this in the build request build, acquiring tools on the fly?
        //
        // or.... on buildrequest::new() where we pass off the args
        // or... on Workspace::new() where we collect the tools from the FS
        // ... yeah probably there where it should be "low impact" and only done at startup
        // orrrrrr on the config loading where it *should* be once
        //
        // we should only have the one build request for the whole serve

        todo!()
    }

    /// Run the build command with a pretty loader, returning the executable output location
    ///
    /// This will also run the fullstack build. Note that fullstack is handled separately within this
    /// code flow rather than outside of it.
    pub(crate) async fn build_all(self) -> Result<BuildArtifacts> {
        tracing::debug!(
            "Running build command... {}",
            if self.force_sequential {
                "(sequentially)"
            } else {
                ""
            }
        );

        todo!()
        // let (app, server) = match self.force_sequential {
        //     true => futures_util::future::try_join(self.cargo_build(), self.build_server()).await?,
        //     false => (self.cargo_build().await?, self.build_server().await?),
        // };

        // AppBundle::new(self, app, server).await
    }

    pub(crate) async fn build_server(&self) -> Result<Option<BuildArtifacts>> {
        tracing::debug!("Building server...");

        todo!()
        // if !self.fullstack {
        //     return Ok(None);
        // }

        // let mut cloned = self.clone();
        // cloned.platform = Platform::Server;

        // Ok(Some(cloned.cargo_build().await?))
    }

    /// The name of the app being served, to display
    pub(crate) fn app_name(&self) -> &str {
        todo!()
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        todo!()
    }
}
