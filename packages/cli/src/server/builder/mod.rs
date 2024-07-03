use crate::cfg::ConfigOptsBuild;
use crate::Result;
use cargo_metadata::diagnostic::Diagnostic;
use dioxus_cli_config::CrateConfig;
use manganis_cli_support::AssetManifest;
use std::{path::PathBuf, time::Duration};
use tokio::process::Child;

mod cargo;
mod prepare_html;
mod progress;
mod web;

// Desktop: native build -> native process
// Web: web build -> web process
// Fullstack web and native build -> native process

struct LiveApplication {
    /// The platform specific process that is running the application
    process: Process,
    /// The websocket message channel that controls the application
    messages: tokio::sync::mpsc::UnboundedSender<Message>,
}

struct WebProcess {
    /// A websocket connection to any running instance of the application
    connections: Vec<Connection>,
    /// The server that is serving the application
    server: Server,
}

struct NativeProcess {
    child: Child,
}

impl NativeProcess {
    async fn kill(&mut self) -> Result<()> {
        Ok(self.child.kill().await?)
    }
}

enum Process {
    /// Running web applications
    Web(WebProcess),
    /// The child process that is building the application
    Native(NativeProcess),
}

impl Process {
    async fn launch(build_result: BuildResult) -> Result<Self> {
        todo!()
    }

    async fn send_message(&self, message: Message) -> Result<()> {
        todo!()
    }

    async fn shutdown(&mut self) -> Result<()> {
        // Try to cleanly shutdown the process first
        self.send_message(Message::Shutdown).await?;

        // Then kill the process
        match self {
            Process::Web(web) => web.kill().await,
            Process::Native(native) => native.kill().await,
        }
    }
}

pub struct BuildRequest {
    /// Whether the build is for serving the application
    pub serve: bool,
    /// Whether to pre-compress assets
    pub precompress_assets: bool,
    /// Whether this is a web build
    pub web: bool,
    /// The configuration for the crate we are building
    pub config: CrateConfig,
    /// The arguments for the build
    pub build_arguments: ConfigOptsBuild,
}

/// A handle to ongoing builds and then the spawned tasks themselves
#[derive(Default)]
pub struct Builder {
    /// The process that is building the application
    build_process: Option<Child>,
}

impl Builder {
    /// Start a new build - killing the current one if it exists
    pub async fn start(&mut self, build_request: BuildRequest) -> Result<BuildResult> {
        // Kill the current build process if it exists
        self.shutdown().await.ok();
    }

    /// Wait for any new updates to the builder - either it completed or gave us a mesage etc
    pub async fn wait(&mut self) {
        todo!()
    }

    /// Shutdown the current build process
    pub(crate) async fn shutdown(&mut self) -> Result<()> {
        if let Some(build_process) = &mut self.build_process {
            build_process.kill().await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub warnings: Vec<Diagnostic>,
    pub executable: Option<PathBuf>,
    pub elapsed_time: Duration,
    pub assets: Option<AssetManifest>,
}
