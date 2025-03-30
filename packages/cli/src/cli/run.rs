use super::*;
use crate::{serve::HandleUpdate, BuildArgs, BuildRequest, Builder, DioxusCrate, Platform, Result};

/// Run the project with the given arguments
#[derive(Clone, Debug, Parser)]
pub(crate) struct RunArgs {
    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) build_args: BuildArgs,
}

impl RunArgs {
    pub(crate) async fn run(self) -> Result<StructuredOutput> {
        let krate = DioxusCrate::new(&self.build_args.args)
            .await
            .context("Failed to load Dioxus workspace")?;

        tracing::trace!("Building crate krate data: {:#?}", krate);
        tracing::trace!("Build args: {:#?}", self.build_args);

        let bundle = Builder::start(&krate, &self.build_args)?.finish().await?;

        let devserver_ip = "127.0.0.1:8081".parse().unwrap();
        let fullstack_ip = "127.0.0.1:8080".parse().unwrap();

        if bundle.build.platform == Platform::Web || bundle.build.fullstack {
            tracing::info!("Serving at: {}", fullstack_ip);
        }

        let mut runner = crate::serve::AppRunner::start(&krate);
        runner
            .open(bundle, devserver_ip, Some(fullstack_ip), true)
            .await?;

        // Run the app, but mostly ignore all the other messages
        // They won't generally be emitted
        loop {
            match runner.running.as_mut().unwrap().wait().await {
                HandleUpdate::StderrReceived { platform, msg } => {
                    tracing::info!("[{platform}]: {msg}")
                }
                HandleUpdate::StdoutReceived { platform, msg } => {
                    tracing::info!("[{platform}]: {msg}")
                }
                HandleUpdate::ProcessExited { platform, status } => {
                    runner.cleanup().await;
                    tracing::info!("[{platform}]: process exited with status: {status:?}");
                    break;
                }
            }
        }

        Ok(StructuredOutput::Success)
    }
}
