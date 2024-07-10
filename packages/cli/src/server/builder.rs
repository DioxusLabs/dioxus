use crate::builder::BuildRequest;
use crate::builder::BuildResult;
use crate::builder::UpdateBuildProgress;
use crate::dioxus_crate::DioxusCrate;
use crate::serve::Serve;
use crate::Result;
use dioxus_cli_config::Platform;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::stream::select_all;
use futures_util::StreamExt;
use tokio::process::ChildStderr;
use tokio::process::ChildStdout;
use tokio::{
    process::{Child, Command},
    task::JoinHandle,
};

/// A handle to ongoing builds and then the spawned tasks themselves
pub struct Builder {
    /// The results of the build
    build_results: Option<JoinHandle<Result<Vec<BuildResult>>>>,
    /// The progress of the builds
    build_progress: Vec<(Platform, UnboundedReceiver<UpdateBuildProgress>)>,
    /// The application we are building
    config: DioxusCrate,
    /// The arguments for the build
    serve: Serve,

    /// The children of the build process
    children: Vec<Child>,
}

impl Builder {
    /// Create a new builder
    pub fn new(config: &DioxusCrate, serve: &Serve) -> Self {
        let serve = serve.clone();
        let config = config.clone();
        Self {
            build_results: None,
            build_progress: Vec::new(),
            config: config.clone(),
            serve,
            children: Vec::new(),
        }
    }

    /// Start a new build - killing the current one if it exists
    pub fn build(&mut self) {
        self.shutdown();
        let build_requests =
            BuildRequest::create(false, &self.config, self.serve.build_arguments.clone());

        let mut set = tokio::task::JoinSet::new();
        for build_request in build_requests {
            let (tx, rx) = futures_channel::mpsc::unbounded();
            self.build_progress.push((
                build_request.build_arguments.platform.unwrap_or_default(),
                rx,
            ));
            set.spawn(async move { build_request.build(tx).await });
        }

        self.build_results = Some(tokio::spawn(async move {
            let mut all_results = Vec::new();
            while let Some(result) = set.join_next().await {
                all_results.push(result.map_err(|err| {
                    crate::Error::Unique(format!("Panic while building project: {err:?}"))
                })??);
            }
            Ok(all_results)
        }));
    }

    /// Wait for any new updates to the builder - either it completed or gave us a message etc
    pub async fn wait(&mut self) -> Result<BuildUpdate> {
        let Some(results) = self.build_results.as_mut() else {
            std::future::pending::<()>().await;
            unreachable!()
        };

        // Wait for build progress
        let mut next = select_all(
            self.build_progress
                .iter_mut()
                .map(|(platform, rx)| rx.map(move |update| (*platform, update))),
        );

        // Wait for the next build result
        tokio::select! {
            application = results => {
                // If we have a build result, open it
                let application = application.map_err(|e| crate::Error::Unique("Build join failed".to_string()))??;

                let mut handles = Vec::new();
                for build_result in &application {
                    if let Some(mut child) = build_result.open(&self.serve.server_arguments)? {
                        let handle = ProcessHandle {
                            stdout: child.stdout.take().unwrap(),
                            stderr: child.stderr.take().unwrap(),
                        };
                        handles.push(handle);
                        self.children.push(child);
                    }
                }

                self.build_results = None;
                Ok(BuildUpdate::BuildFinished(handles))
            }
            progress = next.next() => {
                // If we have a build progress, send it to the screen
                if let Some((platform, update)) = progress {
                    Ok(BuildUpdate::BuildProgress { platform, update })
                }
                else {
                    std::future::pending::<()>().await;
                    unreachable!()
                }
            }
        }
    }

    /// Shutdown the current build process
    pub(crate) fn shutdown(&mut self) {
        for mut child in self.children.drain(..) {
            // Gracefully shtudown the desktop app
            // It might have a receiver to do some cleanup stuff
            if let Some(pid) = child.id() {
                // on unix, we can send a signal to the process to shut down
                #[cfg(unix)]
                {
                    _ = Command::new("kill")
                        .args(["-s", "TERM", &pid.to_string()])
                        .spawn();
                }

                // on windows, use the `taskkill` command
                #[cfg(windows)]
                {
                    _ = Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .spawn();
                }
            }

            // Todo: add a timeout here to kill the process if it doesn't shut down within a reasonable time
            _ = child.start_kill();
        }

        if let Some(tasks) = self.build_results.take() {
            tasks.abort();
        }
        self.build_progress.clear();
    }
}

pub(crate) enum BuildUpdate {
    BuildFinished(Vec<ProcessHandle>),
    BuildProgress {
        platform: Platform,
        update: UpdateBuildProgress,
    },
}

pub(crate) struct ProcessHandle {
    pub(crate) stdout: ChildStdout,
    pub(crate) stderr: ChildStderr,
}
