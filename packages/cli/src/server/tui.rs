use dioxus_cli_config::CrateConfig;

use crate::{build, cfg::ConfigOptsServe};

use super::{BuildEngine, DevServer, FileWatcher};

pub struct TuiOutput {}

pub enum TuiInput {
    Shutdown,
    Keydown,
}

impl TuiOutput {
    pub fn start(cfg: &ConfigOptsServe, crate_config: &CrateConfig) -> Self {
        // Wire the handler to ping the handle_input
        // This will give us some time to handle the input
        ctrlc::set_handler(|| {
            //
        });

        Self {}
    }

    pub async fn wait(&mut self) -> TuiInput {
        todo!()
    }

    pub fn handle_input(&mut self, input: TuiInput) {}

    pub fn draw(
        &self,
        cfg: &ConfigOptsServe,
        crate_config: &CrateConfig,
        build_engine: &BuildEngine,
        server: &DevServer,
        watcher: &FileWatcher,
    ) {
        println!("Drawing the screen");
    }
}
