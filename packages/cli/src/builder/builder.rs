use crate::build::BuildArgs;
use crate::builder::*;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use futures_util::StreamExt;
use progress::{ProgressRx, ProgressTx, UpdateBuildProgress};
use tokio::task::JoinSet;

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// The application we are building
    pub krate: DioxusCrate,

    /// Ongoing builds
    pub building: JoinSet<(Platform, Result<BuildResult>)>,

    /// Messages from the build engine will be sent to this channel
    pub channel: (ProgressTx, ProgressRx),
}

pub enum BuildUpdate {
    Progress(UpdateBuildProgress),

    BuildReady {
        target: Platform,
        result: BuildResult,
    },

    BuildFailed {
        target: Platform,
        err: crate::Error,
    },

    /// All builds have finished and there's nothing left to do
    Finished,
}

impl Builder {
    /// Create a new builder that can accept multiple simultaneous builds
    pub fn new(krate: &DioxusCrate) -> Self {
        Self {
            channel: futures_channel::mpsc::unbounded(),
            krate: krate.clone(),
            building: Default::default(),
        }
    }

    /// Create a new builder and immediately start a build
    pub fn start(krate: &DioxusCrate, args: BuildArgs) -> Result<Self> {
        let mut builder = Self::new(krate);
        builder.build(args)?;
        Ok(builder)
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&mut self, args: BuildArgs) -> Result<()> {
        self.abort_all();

        let mut requests = vec![
            // At least one request for the target app
            BuildRequest::new(self.krate.clone(), args.clone(), self.channel.0.clone()),
        ];

        // And then the fullstack app if we're building a fullstack app
        if args.fullstack {
            super::profiles::initialize_profiles(&self.krate)?;
            let server = BuildRequest::new_server(&self.krate, args.clone(), self.tx());
            requests.push(server);
        }

        // Queue the builds on the joinset
        for build_request in requests {
            let platform = build_request.platform();
            self.building.spawn(async move {
                // Run the build, but in a protected spawn, ensuring we can't produce panics and thus, joinerrors
                let res = tokio::spawn(build_request.build())
                    .await
                    .unwrap_or_else(|err| {
                        Err(crate::Error::Unique(format!(
                            "Panic while building project: {err:?}"
                        )))
                    });

                (platform, res)
            });
        }

        Ok(())
    }

    /// Wait for the build to finish
    pub async fn wait_for_finish(&mut self) {
        loop {
            let next = self.wait().await;
            if let BuildUpdate::Finished = next {
                return;
            }
        }
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    ///
    /// Also listen for any input from the app's handle
    ///
    /// Returns immediately with `Finished` if there are no more builds to run - don't poll-loop this!
    pub async fn wait(&mut self) -> BuildUpdate {
        if self.building.is_empty() {
            return BuildUpdate::Finished;
        }

        tokio::select! {
            Some(update) = self.channel.1.next() => BuildUpdate::Progress(update),
            Some(Ok((target, result))) = self.building.join_next() => {
                match result {
                    Ok(result) => BuildUpdate::BuildReady { target, result },
                    Err(err) => BuildUpdate::BuildFailed { target, err },
                }
            }
        }
    }

    /// Shutdown the current build process
    ///
    /// todo: might want to use a cancellation token here to allow cleaner shutdowns
    pub fn abort_all(&mut self) {
        self.building.abort_all();
    }

    fn tx(&self) -> ProgressTx {
        self.channel.0.clone()
    }
}
