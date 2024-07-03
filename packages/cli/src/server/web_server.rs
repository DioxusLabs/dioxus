use dioxus_cli_config::CrateConfig;

use crate::cfg::ConfigOptsServe;

pub struct WsServer {
    server: tokio::task::JoinHandle<()>,
}

impl WsServer {
    pub fn start(cfg: &ConfigOptsServe, crate_config: &CrateConfig) -> Self {
        Self {}
    }

    pub async fn wait(&mut self) {
        todo!()
    }
}
