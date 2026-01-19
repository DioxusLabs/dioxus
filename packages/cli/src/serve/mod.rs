use crate::{
    styles::{GLOW_STYLE, LINK_STYLE},
    AppBuilder, BuildId, BuildMode, BuilderUpdate, BundleFormat, Result, ServeArgs,
    TraceController,
};

mod ansi_buffer;
mod output;
mod proxy;
mod proxy_ws;
mod runner;
mod server;
mod update;

use anyhow::bail;
use dioxus_dx_wire_format::BuildStage;
pub(crate) use output::*;
pub(crate) use runner::*;
pub(crate) use server::*;
pub(crate) use update::*;

/// For *all* builds, the CLI spins up a dedicated webserver, file watcher, and build infrastructure to serve the project.
///
/// This includes web, desktop, mobile, fullstack, etc.
///
/// Platform specifics:
/// -------------------
/// - Web:         We need to attach a filesystem server to our devtools webserver to serve the project. We
///                want to emulate GithubPages here since most folks are deploying there and expect things like
///                basepath to match.
/// - Desktop:     We spin up the dev server but without a filesystem server.
/// - Mobile:      Basically the same as desktop.
///
/// When fullstack is enabled, we'll also build for the `server` target and then hotreload the server.
/// The "server" is special here since "fullstack" is functionally just an addition to the regular client
/// setup.
///
/// Todos(Jon):
/// - I'd love to be able to configure the CLI while it's running so we can change settings on the fly.
/// - I want us to be able to detect a `server_fn` in the project and then upgrade from a static server
///   to a dynamic one on the fly.
pub(crate) async fn serve_all(args: ServeArgs, tracer: &TraceController) -> Result<()> {
    // Load the args into a plan, resolving all tooling, build dirs, arguments, decoding the multi-target, etc
    let exit_on_error = args.exit_on_error;
    let mut builder = AppServer::new(args).await?;
    let mut devserver = WebServer::start(&builder)?;
    let mut screen = Output::start(builder.interactive).await?;

    // This is our default splash screen. We might want to make this a fancier splash screen in the future
    // Also, these commands might not be the most important, but it's all we've got enabled right now
    tracing::info!(
        r#"-----------------------------------------------------------------
                Serving your app: {binname}! 🚀
                • Press {GLOW_STYLE}`ctrl+c`{GLOW_STYLE:#} to exit the server
                • Press {GLOW_STYLE}`r`{GLOW_STYLE:#} to rebuild the app
                • Press {GLOW_STYLE}`p`{GLOW_STYLE:#} to toggle automatic rebuilds
                • Press {GLOW_STYLE}`v`{GLOW_STYLE:#} to toggle verbose logging
                • Press {GLOW_STYLE}`/`{GLOW_STYLE:#} for more commands and shortcuts{extra}
               ----------------------------------------------------------------"#,
        binname = builder.client.build.executable_name(),
        extra = if builder.client.build.using_dioxus_explicitly {
            format!(
                "\n                Learn more at {LINK_STYLE}https://dioxuslabs.com/learn/0.7/getting_started{LINK_STYLE:#}"
            )
        } else {
            String::new()
        }
    );

    builder.initialize();

    loop {
        // Draw the state of the server to the screen
        screen.render(&builder, &devserver);

        // And then wait for any updates before redrawing
        let msg = tokio::select! {
            msg = builder.wait() => msg,
            msg = devserver.wait() => msg,
            msg = screen.wait() => msg,
            msg = tracer.wait() => msg,
        };

        match msg {
            ServeUpdate::FilesChanged { files } => {
                if files.is_empty() || !builder.hot_reload {
                    continue;
                }

                builder.handle_file_change(&files, &mut devserver).await;
            }

            ServeUpdate::RequestRebuild => {
                // The spacing here is important-ish: we want
                // `Full rebuild:` to line up with
                // `Hotreloading:` to keep the alignment during long edit sessions
                // `Hot-patching:` to keep the alignment during long edit sessions
                tracing::info!("Full rebuild: triggered manually");
                builder.full_rebuild().await;
                devserver.send_reload_start().await;
                devserver.start_build().await;
            }

            // Run the server in the background
            // Waiting for updates here lets us tap into when clients are added/removed
            ServeUpdate::NewConnection {
                id,
                aslr_reference,
                pid,
            } => {
                devserver
                    .send_hotreload(builder.applied_hot_reload_changes(BuildId::PRIMARY))
                    .await;

                if builder.server.is_some() {
                    devserver
                        .send_hotreload(builder.applied_hot_reload_changes(BuildId::SECONDARY))
                        .await;
                }

                builder.client_connected(id, aslr_reference, pid).await;
            }

            // Received a message from the devtools server - currently we only use this for
            // logging, so we just forward it the tui
            ServeUpdate::WsMessage { msg, bundle } => {
                screen.push_ws_message(bundle, &msg);
            }

            // Wait for logs from the build engine
            // These will cause us to update the screen
            // We also can check the status of the builds here in case we have multiple ongoing builds
            ServeUpdate::BuilderUpdate { id, update } => {
                let bundle_format = builder.get_build(id).unwrap().build.bundle;

                // Queue any logs to be printed if need be
                screen.new_build_update(&update);

                // And then update the websocketed clients with the new build status in case they want it
                devserver.new_build_update(&update).await;

                // Start the SSG build if we need to
                builder.new_build_update(&update, &devserver).await;

                // And then open the app if it's ready
                match update {
                    BuilderUpdate::Progress {
                        stage: BuildStage::Failed,
                    } => {
                        if exit_on_error {
                            bail!("Build failed for platform: {bundle_format}");
                        }
                    }
                    BuilderUpdate::Progress {
                        stage: BuildStage::Aborted,
                    } => {
                        if exit_on_error {
                            bail!("Build aborted for platform: {bundle_format}");
                        }
                    }
                    BuilderUpdate::Progress { .. } => {}
                    BuilderUpdate::CompilerMessage { message } => {
                        screen.push_cargo_log(message);
                    }
                    BuilderUpdate::BuildFailed { err } => {
                        tracing::error!(
                            "{ERROR_STYLE}Build failed{ERROR_STYLE:#}: {}",
                            crate::error::log_stacktrace(&err, 15),
                            ERROR_STYLE = crate::styles::ERROR_STYLE,
                        );

                        if exit_on_error {
                            return Err(err);
                        }
                    }
                    BuilderUpdate::BuildReady { bundle } => {
                        match bundle.mode {
                            BuildMode::Thin { ref cache, .. } => {
                                if let Err(err) =
                                    builder.hotpatch(&bundle, id, cache, &mut devserver).await
                                {
                                    tracing::error!("Failed to hot-patch app: {err:?}");

                                    if let Some(_patching) =
                                        err.downcast_ref::<crate::build::PatchError>()
                                    {
                                        tracing::info!("Starting full rebuild: {err:?}");
                                        builder.full_rebuild().await;
                                        devserver.send_reload_start().await;
                                        devserver.start_build().await;
                                    }
                                }
                            }
                            BuildMode::Base { .. } | BuildMode::Fat => {
                                _ = builder
                                    .open(&bundle, &mut devserver)
                                    .await
                                    .inspect_err(|e| tracing::error!("Failed to open app: {}", e));
                            }
                        }

                        // Process any file changes that were queued while the build was in progress.
                        // This handles tools like stylance, tailwind, or sass that generate files
                        // in response to source changes - those changes would otherwise be lost.
                        let pending = builder.take_pending_file_changes();
                        if !pending.is_empty() {
                            tracing::debug!(
                                "Processing {} pending file changes after build",
                                pending.len()
                            );
                            builder.handle_file_change(&pending, &mut devserver).await;
                        }
                    }
                    BuilderUpdate::StdoutReceived { msg } => {
                        screen.push_stdio(bundle_format, msg, tracing::Level::INFO);
                    }
                    BuilderUpdate::StderrReceived { msg } => {
                        screen.push_stdio(bundle_format, msg, tracing::Level::ERROR);
                    }
                    BuilderUpdate::ProcessExited { status } => {
                        if status.success() {
                            tracing::info!(
                                r#"Application [{bundle_format}] exited gracefully.
               • To restart the app, press `r` to rebuild or `o` to open
               • To exit the server, press `ctrl+c`"#
                            );
                        } else {
                            tracing::error!(
                                "Application [{bundle_format}] exited with error: {status}"
                            );
                            if exit_on_error {
                                bail!("Application [{bundle_format}] exited with error: {status}");
                            }
                        }
                    }
                    BuilderUpdate::ProcessWaitFailed { err } => {
                        tracing::warn!(
                            "Failed to wait for process - maybe it's hung or being debugged?: {err}"
                        );
                        if exit_on_error {
                            return Err(err.into());
                        }
                    }
                }
            }

            ServeUpdate::TracingLog { log } => {
                screen.push_log(log);
            }

            ServeUpdate::OpenApp => match builder.use_hotpatch_engine {
                true if !matches!(builder.client.build.bundle, BundleFormat::Web) => {
                    tracing::warn!(
                        "Opening a native app with hotpatching enabled requires a full rebuild..."
                    );
                    builder.full_rebuild().await;
                    devserver.send_reload_start().await;
                    devserver.start_build().await;
                }
                _ => {
                    if let Err(err) = builder.open_all(&devserver, true).await {
                        tracing::error!(
                            "Failed to open app: {}",
                            crate::error::log_stacktrace(&err, 15)
                        )
                    }
                }
            },

            ServeUpdate::Redraw => {
                // simply returning will cause a redraw
            }

            ServeUpdate::ToggleShouldRebuild => {
                use crate::styles::{ERROR, NOTE_STYLE};
                builder.automatic_rebuilds = !builder.automatic_rebuilds;
                tracing::info!(
                    "Automatic rebuilds are currently: {}",
                    if builder.automatic_rebuilds {
                        format!("{NOTE_STYLE}enabled{NOTE_STYLE:#}")
                    } else {
                        format!("{ERROR}disabled{ERROR:#}")
                    }
                )
            }

            ServeUpdate::OpenDebugger { id } => {
                builder.open_debugger(&devserver, id).await;
            }

            ServeUpdate::Exit { error } => {
                _ = builder.shutdown().await;
                _ = devserver.shutdown().await;

                match error {
                    Some(err) => return Err(err),
                    None => return Ok(()),
                }
            }
        }
    }
}
