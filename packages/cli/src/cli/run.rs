use crate::{build::TargetArgs, builder::Builder, serve::ServeUpdate};
use anyhow::Context;
use build::BuildArgs;
use futures_util::{stream::FuturesUnordered, StreamExt};
use std::{path::Path, process::exit};

use crate::DioxusCrate;

use super::*;

/// Check the Rust files in the project for issues.
#[derive(Clone, Debug, Parser)]
pub(crate) struct RunArgs {
    /// Information about the target to check
    #[clap(flatten)]
    pub(crate) build_args: BuildArgs,
}

impl RunArgs {
    pub(crate) async fn run(mut self) -> anyhow::Result<()> {
        let mut dioxus_crate = DioxusCrate::new(&self.build_args.target_args)
            .context("Failed to load Dioxus workspace")?;

        self.build_args.resolve(&mut dioxus_crate)?;

        let bundles = Builder::start(&mut dioxus_crate, self.build_args.clone())?
            .wait_for_finish()
            .await?;

        let mut runner = crate::serve::AppRunner::start();

        let devserver_ip = "127.0.0.1:8080".parse().unwrap();
        let fullstack_ip = "127.0.0.1:6955".parse().unwrap();

        for bundle in bundles {
            runner.open(bundle, devserver_ip, Some(fullstack_ip)).await;
        }

        loop {
            let msg = runner.wait().await;

            match msg {
                ServeUpdate::StderrReceived { platform, msg } => println!("[{platform}]: {msg}"),
                ServeUpdate::StdoutReceived { platform, msg } => println!("[{platform}]: {msg}"),
                ServeUpdate::ProcessExited { platform, status } => {
                    runner.kill(platform).await;
                    eprintln!("[{platform}]: process exited with status: {status:?}")
                }

                ServeUpdate::TracingLog { log } => todo!(),
                ServeUpdate::NewConnection => todo!(),
                ServeUpdate::WsMessage(_) => todo!(),
                ServeUpdate::BuildUpdate(_) => todo!(),
                ServeUpdate::FilesChanged { files } => todo!(),
                ServeUpdate::TuiInput { event } => todo!(),
            }
        }

        Ok(())
    }
}
