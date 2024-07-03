use crate::Result;
use dioxus_cli_config::CrateConfig;

use crate::cfg::ConfigOptsServe;

pub struct BuildEngine {}

impl BuildEngine {
    /// Starting the build engine will also start the build process
    pub fn start(cfg: &ConfigOptsServe, crate_config: &CrateConfig) -> Self {
        Self {}
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
