use super::*;
use crate::{
    serve::{AppServer, ServeUpdate, WebServer},
    BuilderUpdate, Platform, Result,
};
use dioxus_dx_wire_format::BuildStage;

/// Run the project with the given arguments
///
/// This is a shorthand for `dx serve` with interactive mode and hot-reload disabled.
#[derive(Clone, Debug, Parser)]
pub(crate) struct RunArgs {
    /// Information about the target to build
    #[clap(flatten)]
    pub(crate) args: ServeArgs,
}

impl RunArgs {
    pub(crate) async fn run(mut self) -> Result<StructuredOutput> {
        // Override the build arguments, leveraging our serve infrastructure.
        //
        // We want to turn off the fancy stuff like the TUI, watcher, and hot-reload, but leave logging
        // and other things like the devserver on.
        self.args.hot_patch = false;
        self.args.interactive = Some(false);
        self.args.hot_reload = Some(false);
        self.args.watch = Some(false);

        let mut builder = AppServer::start(self.args).await?;
        let mut devserver = WebServer::start(&builder)?;

        loop {
            let msg = tokio::select! {
                msg = builder.wait() => msg,
                msg = devserver.wait() => msg,
            };

            match msg {
                // Wait for logs from the build engine
                // These will cause us to update the screen
                // We also can check the status of the builds here in case we have multiple ongoing builds
                ServeUpdate::BuilderUpdate { id, update } => {
                    let platform = builder.get_build(id).unwrap().build.platform;

                    // And then update the websocketed clients with the new build status in case they want it
                    devserver.new_build_update(&update).await;

                    // And then open the app if it's ready
                    match update {
                        BuilderUpdate::BuildReady { bundle } => {
                            _ = builder
                                .open(bundle, &mut devserver)
                                .await
                                .inspect_err(|e| tracing::error!("Failed to open app: {}", e));

                            if platform == Platform::Web {
                                tracing::info!(
                                    "Serving app at http://{}:{}",
                                    builder.devserver_bind_ip,
                                    builder.devserver_port
                                );
                            }
                        }
                        BuilderUpdate::Progress { stage } => match stage {
                            BuildStage::Initializing => {
                                tracing::info!("[{platform}] Initializing build")
                            }
                            BuildStage::Starting { .. } => {}
                            BuildStage::InstallingTooling => {}
                            BuildStage::Compiling {
                                current,
                                total,
                                krate,
                            } => {
                                tracing::info!("[{platform}] Compiling {krate} ({current}/{total})",)
                            }
                            BuildStage::RunningBindgen => {
                                tracing::info!("[{platform}] Running WASM bindgen")
                            }
                            BuildStage::SplittingBundle => {}
                            BuildStage::OptimizingWasm => {
                                tracing::info!("[{platform}] Optimizing WASM with `wasm-opt`")
                            }
                            BuildStage::Linking => tracing::info!("Linking app"),
                            BuildStage::Hotpatching => todo!(),
                            BuildStage::CopyingAssets {
                                current,
                                total,
                                path,
                            } => tracing::info!(
                                "[{platform}] Copying asset {} ({current}/{total})",
                                path.display(),
                            ),
                            BuildStage::Bundling => tracing::info!("[{platform}] Bundling app"),
                            BuildStage::RunningGradle => {
                                tracing::info!("[{platform}] Running Gradle")
                            }
                            BuildStage::Success => {}
                            BuildStage::Failed => {}
                            BuildStage::Aborted => {}
                            BuildStage::Restarting => {}
                            BuildStage::CompressingAssets => {}
                            _ => {}
                        },
                        BuilderUpdate::CompilerMessage { message } => {
                            print!("{}", message);
                        }
                        BuilderUpdate::BuildFailed { err } => {
                            tracing::error!("Build failed: {}", err);
                        }
                        BuilderUpdate::StdoutReceived { msg } => {
                            tracing::info!("[{platform}] {msg}");
                        }
                        BuilderUpdate::StderrReceived { msg } => {
                            tracing::error!("[{platform}] {msg}");
                        }
                        BuilderUpdate::ProcessExited { status } => {
                            if !status.success() {
                                tracing::error!(
                                    "Application [{platform}] exited with error: {status}"
                                );
                            }

                            break;
                        }
                        BuilderUpdate::ProcessWaitFailed { .. } => {}
                    }
                }
                _ => {}
            }
        }

        Ok(StructuredOutput::Success)
    }
}
