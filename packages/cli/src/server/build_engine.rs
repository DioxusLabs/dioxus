use crate::Result;
use dioxus_cli_config::CrateConfig;
use tokio::process::Child;

use crate::cfg::ConfigOptsServe;

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct BuildEngine {
    desktop_app: Option<Child>,
    mobile_app: Option<Child>,
}

impl BuildEngine {
    /// Starting the build engine will also start the build process
    pub fn start(cfg: &ConfigOptsServe, crate_config: &CrateConfig) -> Self {
        Self {
            desktop_app: None,
            mobile_app: None,
        }
    }

    /// Wait for any new updates to the builder - either it completed or gave us a mesage etc
    pub async fn wait(&mut self) {
        todo!()
    }

    /// Initiate a new build, killing the old one if it exists
    pub fn queue_build(&mut self) {}

    pub(crate) async fn shutdown(&self) -> Result<()> {
        todo!()
    }
}
