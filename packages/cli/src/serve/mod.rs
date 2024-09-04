use crate::builder::{BuildUpdate, Builder, Platform, Stage, UpdateBuildProgress, UpdateStage};
use crate::cli::serve::ServeArgs;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;

mod detect;
mod hot_reloading_file_map;
mod logs_tab;
mod output;
mod proxy;
mod runner;
mod server;
mod update;
mod util;
mod watcher;

use output::*;
use runner::*;
use server::*;
use update::*;
use util::*;
use watcher::*;

/// For *all* builds, the CLI spins up a dedicated webserver, file watcher, and build infrastructure to serve the project.
///
/// This includes web, desktop, mobile, fullstack, etc.
///
/// Platform specifics:
/// -------------------
/// - Web:         we need to attach a filesystem server to our devtools webserver to serve the project. We
///                want to emulate GithubPages here since most folks are deploying there and expect things like
///                basepath to match.
/// - Fullstack:   We spin up the same dev server but in this case the fullstack server itself needs to
///                proxy all dev requests to our dev server. Todo: we might just want "web" to use the
///                fullstack server by default, such that serving web on non-wasm platforms is just a
///                serving a proper server.
/// - Desktop:     We spin up the dev server but without a filesystem server.
/// - Mobile:      Basically the same as desktop.
///
///
/// Notes:
/// - All filesystem changes are tracked here
/// - We send all updates to connected websocket connections. Even desktop connects via the websocket
/// - Right now desktop compiles tokio-tungstenite to do the connection but we could in theory reuse
///   the websocket logic from the webview for thinner builds.
///
/// Todos(Jon):
/// - I'd love to be able to configure the CLI while it's running so we can change settings on the fly.
/// - Handle logs from the build engine separately?
/// - I want us to be able to detect a `server_fn` in the project and then upgrade from a static server
///   to a dynamic one on the fly.
pub async fn serve_all(args: ServeArgs, krate: DioxusCrate) -> Result<()> {
    // Start each component of the devserver.
    // Start the screen first, since it'll start to swallow the tracing logs from the rest of the the cli
    let mut screen = Output::start(&args).expect("Failed to open terminal logger");

    // Note that starting the builder will queue up a build immediately
    let mut builder = Builder::start(&krate, args.build_arguments.clone())?;

    // The watcher and devserver are started after but don't really matter in order
    let mut devserver = DevServer::start(&args, &krate);
    let mut watcher = Watcher::start(&args, &krate);
    let mut runner = AppRunner::start(&args, &krate);

    loop {
        // Make sure we don't hog the CPU: these loop { select! {} } blocks can starve the executor if we're not careful
        tokio::task::yield_now().await;

        // Draw the state of the server to the screen
        screen.render(&args, &krate, &builder, &devserver, &watcher);

        // And then wait for any updates before redrawing
        let msg = tokio::select! {
            msg = builder.wait() => ServeUpdate::BuildUpdate(msg),
            msg = watcher.wait() => msg,
            msg = devserver.wait() => msg,
            msg = screen.wait() => msg,
            msg = runner.wait() => msg,
        };

        match msg {
            ServeUpdate::FilesChanged { files } => {
                if files.is_empty() || args.should_hotreload() {
                    continue;
                }

                // if change is hotreloadable, hotreload it
                // and then send that update to all connected clients
                if let Some(hr) = watcher.attempt_hot_reload(&krate, files) {
                    // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                    if hr.templates.is_empty() && hr.assets.is_empty() {
                        continue;
                    }

                    devserver.send_hotreload(hr).await;
                } else {
                    // We're going to kick off a new build, interrupting the current build if it's ongoing
                    builder.build(args.build_arguments.clone())?;

                    // Clear the hot reload changes
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
                screen.new_ws_message(Platform::Web, msg);
            }

            // Wait for logs from the build engine
            // These will cause us to update the screen
            // We also can check the status of the builds here in case we have multiple ongoing builds
            ServeUpdate::BuildUpdate(BuildUpdate::Progress(update)) => {
                let update_clone = update.clone();
                screen.new_build_logs(update.platform, update_clone);
                devserver
                    .update_build_status(screen.build_progress.progress(), update.stage.to_string())
                    .await;

                match update {
                    // Send rebuild start message.
                    UpdateBuildProgress {
                        stage: Stage::Compiling,
                        update: UpdateStage::Start,
                        platform: _,
                    } => devserver.send_reload_start().await,

                    // Send rebuild failed message.
                    UpdateBuildProgress {
                        stage: Stage::Finished,
                        update: UpdateStage::Failed(_),
                        platform: _,
                    } => devserver.send_reload_failed().await,

                    _ => {}
                }
            }

            ServeUpdate::BuildUpdate(BuildUpdate::BuildFailed { err, target }) => {
                devserver.send_build_error(err).await;
            }

            ServeUpdate::BuildUpdate(BuildUpdate::BuildReady { target, result }) => {
                // if !results.is_empty() {
                //     builder.children.clear();
                // }

                // // If we have a build result, open it
                // for build_result in results.iter() {
                //     let child = server.open(build_result);

                //     match child {
                //         Ok(Some(child_proc)) => builder
                //             .children
                //             .push((build_result.target_platform, child_proc)),
                //         Err(e) => {
                //             tracing::error!("Failed to open build result: {e}");
                //             break;
                //         }
                //         _ => {}
                //     }
                // }

                // // Make sure we immediately capture the stdout/stderr of the executable -
                // // otherwise it'll clobber our terminal output
                // screen.new_ready_app(&mut builder, results);

                // // And then finally tell the server to reload
                // server.send_reload_command().await;
            }

            ServeUpdate::StdoutReceived { target, msg } => {}

            ServeUpdate::StderrReceived { target, msg } => {}

            // If the process exited *cleanly*, we can exit
            ServeUpdate::ProcessExited {
                status,
                target_platform,
            } => {
                todo!("process exited")
                // // Then remove the child process
                // builder
                //     .children
                //     .retain(|(platform, _)| *platform != target_platform);
                // match status {
                //     Ok(status) => {
                //         if status.success() {
                //             break;
                //         } else {
                //             tracing::error!("Application exited with status: {status}");
                //         }
                //     }
                //     Err(e) => {
                //         tracing::error!("Application exited with error: {e}");
                //     }
                // }
            }

            // Handle TUI input and maybe even rebuild the app
            ServeUpdate::TuiInput { rebuild } => {
                if rebuild {
                    builder.build(args.build_arguments.clone())?;
                    devserver.start_build().await
                }
            }

            // A fatal error occured and we need to exit + cleanup
            ServeUpdate::Fatal { err } => break,
        }
    }

    // Kill the clients first
    _ = devserver.shutdown().await;
    _ = screen.shutdown();
    _ = builder.abort_all();

    Ok(())
}
