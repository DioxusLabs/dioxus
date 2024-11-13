use super::*;
use crate::{serve::ServeUpdate, BuildArgs, Builder, DioxusCrate, Result};

/// Run the project with the given arguments
#[derive(Clone, Debug, Parser)]
pub(crate) struct RunArgs {
    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) build_args: BuildArgs,
}

impl RunArgs {
    pub(crate) async fn run(mut self) -> Result<StructuredOutput> {
        let krate = DioxusCrate::new(&self.build_args.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.build_args.resolve(&krate)?;

        tracing::trace!("Building crate krate data: {:#?}", krate);
        tracing::trace!("Build args: {:#?}", self.build_args);

        let bundle = Builder::start(&krate, self.build_args.clone())?
            .finish()
            .await?;

        let devserver_ip = "127.0.0.1:8080".parse().unwrap();
        let fullstack_ip = "127.0.0.1:8081".parse().unwrap();

        let mut runner = crate::serve::AppRunner::start(&krate);
        runner
            .open(bundle, devserver_ip, Some(fullstack_ip), true)
            .await?;

        // Run the app, but mostly ignore all the other messages
        // They won't generally be emitted
        loop {
            match runner.wait().await {
                ServeUpdate::StderrReceived { platform, msg } => {
                    tracing::info!("[{platform}]: {msg}")
                }
                ServeUpdate::StdoutReceived { platform, msg } => {
                    tracing::info!("[{platform}]: {msg}")
                }
                ServeUpdate::ProcessExited { platform, status } => {
                    runner.kill(platform);
                    tracing::info!("[{platform}]: process exited with status: {status:?}");
                    break;
                }
                ServeUpdate::BuildUpdate { .. } => {}
                ServeUpdate::TracingLog { .. } => {}
                ServeUpdate::Exit { .. } => break,
                ServeUpdate::NewConnection => {}
                ServeUpdate::WsMessage(_) => {}
                ServeUpdate::FilesChanged { .. } => {}
                ServeUpdate::RequestRebuild => {}
                ServeUpdate::Redraw => {}
                ServeUpdate::OpenApp => {}
                ServeUpdate::ToggleShouldRebuild => {}
            }
        }

        Ok(StructuredOutput::Success)
    }
}
