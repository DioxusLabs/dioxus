use super::*;
use crate::{AppBuilder, BuildArgs, BuildRequest, Platform, Result};

/// Run the project with the given arguments
#[derive(Clone, Debug, Parser)]
pub(crate) struct RunArgs {
    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) build_args: BuildArgs,
}

impl RunArgs {
    pub(crate) async fn run(self) -> Result<StructuredOutput> {
        let build = BuildRequest::new(&self.build_args)
            .await
            .context("error building project")?;

        let mut builder = AppBuilder::start(&build)?;
        let artifacts = builder.finish_build().await?;

        let devserver_ip = "127.0.0.1:8081".parse().unwrap();
        let fullstack_ip = "127.0.0.1:8080".parse().unwrap();
        let mut open_address = None;

        todo!();
        // if build.platform == Platform::Web || build.fullstack {
        //     tracing::info!("Serving at: {}", fullstack_ip);
        // }

        builder
            .open(devserver_ip, Some(fullstack_ip), true, false)
            .await?;

        todo!();
        // // Run the app, but mostly ignore all the other messages
        // // They won't generally be emitted
        // loop {
        //     match builder.wait().await {
        //         HandleUpdate::StderrReceived { platform, msg } => {
        //             tracing::info!("[{platform}]: {msg}")
        //         }
        //         HandleUpdate::StdoutReceived { platform, msg } => {
        //             tracing::info!("[{platform}]: {msg}")
        //         }
        //         HandleUpdate::ProcessExited { platform, status } => {
        //             builder.cleanup().await;
        //             tracing::info!("[{platform}]: process exited with status: {status:?}");
        //             break;
        //         }
        //     }
        // }

        Ok(StructuredOutput::Success)
    }
}
