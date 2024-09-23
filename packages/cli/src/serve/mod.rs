use crate::{
    BuildStage, BuildUpdate, Builder, DioxusCrate, Platform, Result, ServeArgs, TraceController,
    TraceSrc,
};

mod ansi_buffer;
mod detect;
mod handle;
mod hot_reloading_file_map;
mod output;
mod proxy;
mod runner;
mod server;
mod update;
mod watcher;

pub(crate) use handle::*;
pub(crate) use output::*;
pub(crate) use runner::*;
pub(crate) use server::*;
pub(crate) use update::*;
pub(crate) use watcher::*;

/// For *all* builds, the CLI spins up a dedicated webserver, file watcher, and build infrastructure to serve the project.
///
/// This includes web, desktop, mobile, fullstack, etc.
///
/// Platform specifics:
/// -------------------
/// - Web:         we need to attach a filesystem server to our devtools webserver to serve the project. We
///                want to emulate GithubPages here since most folks are deploying there and expect things like
///                basepath to match.
/// - Desktop:     We spin up the dev server but without a filesystem server.
/// - Mobile:      Basically the same as desktop.
///
/// When fullstack is enabled, we'll also build for the `server` target and then hotreload the server.
/// The "server" is special here since "fullstack" is functionaly just an addition to the regular client
/// setup.
///
/// Todos(Jon):
/// - I'd love to be able to configure the CLI while it's running so we can change settings on the fly.
/// - I want us to be able to detect a `server_fn` in the project and then upgrade from a static server
///   to a dynamic one on the fly.
pub(crate) async fn serve_all(args: ServeArgs, krate: DioxusCrate) -> Result<()> {
    let mut tracer = TraceController::redirect();

    // Note that starting the builder will queue up a build immediately
    let mut builder = Builder::start(&krate, args.build_args())?;
    let mut devserver = DevServer::start(&args, &krate)?;
    let mut screen = Output::start(&args).expect("Failed to open terminal logger");
    let mut watcher = Watcher::start(&args, &krate);
    let mut runner = AppRunner::start();

    let err: Result<(), crate::Error> = loop {
        // Draw the state of the server to the screen
        screen.render(&args, &krate, &builder, &devserver, &watcher);

        // And then wait for any updates before redrawing
        let msg = tokio::select! {
            msg = builder.wait() => ServeUpdate::BuildUpdate(msg),
            msg = watcher.wait() => msg,
            msg = devserver.wait() => msg,
            msg = screen.wait() => msg,
            msg = runner.wait() => msg,
            msg = tracer.wait() => msg,
        };

        match msg {
            ServeUpdate::FilesChanged { files } => {
                if files.is_empty() || !args.should_hotreload() {
                    continue;
                }

                let file = files[0].display().to_string();
                let file = file.trim_start_matches(&krate.crate_dir().display().to_string());

                // if change is hotreloadable, hotreload it
                // and then send that update to all connected clients
                if let Some(hr) = watcher.attempt_hot_reload(files, &runner) {
                    tracing::info!(dx_src = ?TraceSrc::Dev, "Hotreloading: {} in 3ms", file);

                    // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                    if hr.templates.is_empty()
                        && hr.assets.is_empty()
                        && hr.unknown_files.is_empty()
                    {
                        continue;
                    }

                    devserver.send_hotreload(hr).await;
                } else {
                    tracing::info!(dx_src = ?TraceSrc::Dev, "Full rebuild: {} in 3ms", file);

                    // We're going to kick off a new build, interrupting the current build if it's ongoing
                    builder.rebuild(args.build_arguments.clone());

                    // Clear the hot reload changes so we don't have out-of-sync issues with changed UI
                    watcher.clear_hot_reload_changes();

                    // Tell the server to show a loading page for any new requests
                    devserver.start_build().await;
                }
            }

            // Run the server in the background
            // Waiting for updates here lets us tap into when clients are added/removed
            ServeUpdate::NewConnection => {
                if let Some(msg) = watcher.applied_hot_reload_changes() {
                    devserver.send_hotreload(msg).await;
                }
            }

            // Received a message from the devtools server - currently we only use this for
            // logging, so we just forward it the tui
            ServeUpdate::WsMessage(msg) => {
                screen.push_ws_message(Platform::Web, msg);
            }

            // Wait for logs from the build engine
            // These will cause us to update the screen
            // We also can check the status of the builds here in case we have multiple ongoing builds
            ServeUpdate::BuildUpdate(update) => {
                // Queue any logs to be printed if need be
                screen.new_build_update(&update);

                // And then update the websocketed clients with the new build status in case they want it
                devserver.new_build_update(&update).await;

                // And then open the app if it's ready
                // todo: there might be more things to do here that require coordination with other pieces of the CLI
                // todo: maybe we want to shuffle the runner around to send an "open" command instead of doing that
                match update {
                    BuildUpdate::Progress { .. } => {}
                    BuildUpdate::Message {} => {}
                    BuildUpdate::BuildFailed { .. } => {}
                    BuildUpdate::BuildReady { bundle } => {
                        let handle = runner
                            .open(
                                bundle,
                                devserver.devserver_address(),
                                devserver.server_address(),
                            )
                            .await;

                        match handle {
                            Ok(handle) => {
                                // Make sure we immediately capture the stdout/stderr of the executable -
                                // otherwise it'll clobber our terminal output
                                screen.new_ready_app(handle);

                                // And then finally tell the server to reload
                                devserver.send_reload_command().await;
                            }

                            Err(e) => {
                                tracing::error!("Failed to open app: {}", e);
                            }
                        }
                    }
                }
            }

            // If the process exited *cleanly*, we can exit
            ServeUpdate::ProcessExited { status, platform } => {
                if !status.success() {
                    tracing::error!(dx_src = ?TraceSrc::Dev, "Application [{platform}] exited with status: {status}");
                    break Err(anyhow::anyhow!("Application exited with status: {status}").into());
                }

                runner.kill(platform).await;
            }

            ServeUpdate::StdoutReceived { platform, msg } => {
                screen.push_stdout(platform, msg);
            }

            ServeUpdate::StderrReceived { platform, msg } => {
                screen.push_stderr(platform, msg);
            }

            ServeUpdate::TracingLog { log } => {
                screen.push_inner_log(log);
            }

            ServeUpdate::RequestRebuild => {
                tracing::info!("Full rebuild: triggered manually");
                builder.rebuild(args.build_arguments.clone());
                devserver.start_build().await
            }

            ServeUpdate::OpenApp => {
                runner.open_existing().await;
            }

            ServeUpdate::Redraw => {
                // simply returning will cause a redraw
            }

            ServeUpdate::Exit { error } => match error {
                Some(err) => break Err(anyhow::anyhow!("{}", err).into()),
                None => break Ok(()),
            },
        }
    };

    _ = devserver.shutdown().await;
    _ = screen.shutdown();
    _ = builder.abort_all();
    _ = tracer.shutdown();

    if let Err(err) = err {
        eprintln!("Exiting with error: {}", err);
    }

    Ok(())
}
