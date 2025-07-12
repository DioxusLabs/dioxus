use super::*;
use crate::{
    serve::{AppServer, ServeUpdate, WebServer},
    BuilderUpdate, Platform, Result,
};
use anyhow::bail;
use dioxus_dx_wire_format::BuildStage;

/// Run the project with the given arguments
///
/// This is a shorthand for `dx serve` with interactive mode and hot-reload disabled.
///
/// Unlike `dx serve`, errors during build and run will cascade out as an error, rather than being
/// handled by the TUI, making it more suitable for scripting, automation, or CI/CD pipelines.
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

        let mut builder = AppServer::new(self.args).await?;
        let mut devserver = WebServer::start(&builder)?;

        builder.initialize();

        loop {
            let msg = tokio::select! {
                msg = builder.wait() => msg,
                msg = devserver.wait() => msg,
            };

            match msg {
                ServeUpdate::BuilderUpdate { id, update } => {
                    let platform = builder.get_build(id).unwrap().build.platform;

                    // And then update the websocketed clients with the new build status in case they want it
                    devserver.new_build_update(&update).await;

                    // Finally, we also want to update the builder with the new update
                    builder.new_build_update(&update, &devserver).await;

                    // And then open the app if it's ready
                    match update {
                        BuilderUpdate::BuildReady { bundle } => {
                            _ = builder
                                .open(&bundle, &mut devserver)
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
                                tracing::debug!(
                                    "[{platform}] ({current}/{total}) Compiling {krate} ",
                                )
                            }
                            BuildStage::RunningBindgen => {
                                tracing::info!("[{platform}] Running WASM bindgen")
                            }
                            BuildStage::SplittingBundle => {}
                            BuildStage::OptimizingWasm => {
                                tracing::info!("[{platform}] Optimizing WASM with `wasm-opt`")
                            }
                            BuildStage::Linking => tracing::info!("Linking app"),
                            BuildStage::Hotpatching => {}
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
                            BuildStage::Restarting => {}
                            BuildStage::CompressingAssets => {}
                            BuildStage::ExtractingAssets => {}
                            BuildStage::Prerendering => {
                                tracing::info!("[{platform}] Prerendering app")
                            }
                            BuildStage::Failed => {
                                tracing::error!("[{platform}] Build failed");
                                bail!("Build failed for platform: {platform}");
                            }
                            BuildStage::Aborted => {
                                tracing::error!("[{platform}] Build aborted");
                                bail!("Build aborted for platform: {platform}");
                            }
                            _ => {}
                        },
                        BuilderUpdate::CompilerMessage { message } => {
                            print!("{}", message);
                        }
                        BuilderUpdate::BuildFailed { err } => {
                            tracing::error!("❌ Build failed: {}", err);
                            return Err(err);
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
                                bail!("Application [{platform}] exited with error: {status}");
                            }

                            break;
                        }
                        BuilderUpdate::ProcessWaitFailed { err } => {
                            return Err(err.into());
                        }
                    }
                }
                ServeUpdate::Exit { .. } => break,
                ServeUpdate::NewConnection { .. } => {}
                ServeUpdate::WsMessage { .. } => {}
                ServeUpdate::FilesChanged { .. } => {}
                ServeUpdate::OpenApp => {}
                ServeUpdate::RequestRebuild => {}
                ServeUpdate::ToggleShouldRebuild => {}
                ServeUpdate::OpenDebugger { .. } => {}
                ServeUpdate::Redraw => {}
                ServeUpdate::TracingLog { .. } => {}
            }
        }

        Ok(StructuredOutput::Success)
    }
}
