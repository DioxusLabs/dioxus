use super::{BuildRequest, BuildStage, BuildUpdate, ProgressRx, ProgressTx};
use crate::{build::BuildArgs, Result};
use crate::{bundler::AppBundle, DioxusCrate};
use futures_util::{FutureExt, StreamExt};
use tokio::task::JoinHandle;

/// A handle to an ongoing build that tracks its progress and can be aborted
///
/// This provides the channel for communicating with the build and stores state about the build including
/// its build stage, progress, and any errors and warnings that occured.
pub struct BuildHandle {
    pub request: BuildRequest,

    pub stage: BuildStage,

    pub build: JoinHandle<Result<AppBundle>>,

    /// Messages from the ongoing builds will be sent on this channel
    pub progress: (ProgressTx, ProgressRx),
}

impl BuildHandle {
    pub fn new(krate: &DioxusCrate, args: &BuildArgs) -> Self {
        let progress = futures_channel::mpsc::unbounded();
        let request = BuildRequest::new(&krate, args.clone(), progress.0.clone());
        Self {
            request: request.clone(),
            stage: BuildStage::Initializing,
            build: tokio::spawn(request.build()),
            progress,
        }
    }

    pub fn new_server(krate: &DioxusCrate, args: &BuildArgs) -> Self {
        let progress = futures_channel::mpsc::unbounded();
        let request = BuildRequest::new_server(&krate, args.clone(), progress.0.clone());
        Self {
            request: request.clone(),
            stage: BuildStage::Initializing,
            build: tokio::spawn(request.build()),
            progress,
        }
    }

    /// Wait for the next update from the build
    /// If the build is finished, this will stay pending forever
    pub async fn wait(&mut self) -> BuildUpdate {
        let platform = self.request.platform();

        // let res = tokio::select! {
        //     Some(progress) = self.progress.1.next() => BuildUpdate::Progress {
        //         progress,
        //         stage
        //     },
        //     bundle = (&mut self.build) => {
        //         match bundle {
        //             Ok(Ok(bundle)) => BuildUpdate::BuildReady { bundle },
        //             Ok(Err(err)) => BuildUpdate::BuildFailed { err },
        //             Err(err) => BuildUpdate::BuildFailed { err: anyhow::anyhow!("{err}").into() },
        //         }
        //     },
        //     else => std::future::pending().await,
        // };

        // match &res {
        //     BuildUpdate::Progress(build_update_progress) => todo!(),
        //     BuildUpdate::BuildReady { platform, bundle } => todo!(),
        //     BuildUpdate::BuildFailed { platform, err } => todo!(),
        // }

        // res

        todo!()
    }

    pub fn abort(&mut self) {
        self.build.abort();
    }
}
