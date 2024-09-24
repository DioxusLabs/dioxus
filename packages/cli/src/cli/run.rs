use crate::DioxusCrate;
use crate::{serve::ServeUpdate, Builder};
use anyhow::Context;
use build::BuildArgs;

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

        println!("Building crate krate data: {:#?}", dioxus_crate);

        println!("Build args: {:#?}", self.build_args);

        let bundle = Builder::start(&mut dioxus_crate, self.build_args.clone())?
            .finish()
            .await?;

        let mut runner = crate::serve::AppRunner::start();

        let devserver_ip = "127.0.0.1:8080".parse().unwrap();
        let fullstack_ip = "127.0.0.1:6955".parse().unwrap();

        runner.open(bundle, devserver_ip, Some(fullstack_ip))?;

        loop {
            let msg = runner.wait().await;

            match msg {
                ServeUpdate::StderrReceived { platform, msg } => println!("[{platform}]: {msg}"),
                ServeUpdate::StdoutReceived { platform, msg } => println!("[{platform}]: {msg}"),
                ServeUpdate::ProcessExited { platform, status } => {
                    runner.kill(platform);
                    eprintln!("[{platform}]: process exited with status: {status:?}")
                }
                ServeUpdate::BuildUpdate { .. } => {}
                ServeUpdate::TracingLog { log } => {}
                ServeUpdate::NewConnection => {}
                ServeUpdate::WsMessage(_) => {}
                ServeUpdate::FilesChanged { files } => {}
                ServeUpdate::RequestRebuild => {}
                ServeUpdate::Redraw => {}
                ServeUpdate::Exit { error } => {}
                ServeUpdate::OpenApp => {}
            }
        }

        Ok(())
    }
}
