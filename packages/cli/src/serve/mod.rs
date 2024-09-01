use crate::builder::{Stage, TargetPlatform, UpdateBuildProgress, UpdateStage};
use crate::cli::serve::Serve;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;

mod builder;
mod detect;
mod hot_reloading_file_map;
mod logs_tab;
mod output;
mod proxy;
mod server;
mod update;
mod util;
mod watcher;

use builder::*;
use output::*;
use server::*;
use update::*;
use util::*;
use watcher::*;

/// For *all* builds, the CLI spins up a dedicated webserver, file watcher, and build infrastructure to serve the project.
///
/// This includes web, desktop, mobile, fullstack, etc.
///
/// Platform specifics:
/// - Web:       we need to attach a filesystem server to our devtools webserver to serve the project. We
///              want to emulate GithubPages here since most folks are deploying there and expect things like
///              basepath to match.
/// - Fullstack: We spin up the same dev server but in this case the fullstack server itself needs to
///              proxy all dev requests to our dev server
/// - Desktop:   We spin up the dev server but without a filesystem server.
/// - Mobile:    Basically the same as desktop.
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
pub async fn serve_all(
    serve: Serve,
    krate: DioxusCrate,
    log_control: crate::tracer::CLILogControl,
) -> Result<()> {
    // Start each component of the devserver.
    // Starting the builder will queue up a build
    let mut builder = Builder::start(&krate, &serve)?;
    let mut server = DevServer::start(&serve, &krate);
    let mut watcher = Watcher::start(&serve, &krate);
    let mut screen = Output::start(&serve, log_control).expect("Failed to open terminal logger");

    loop {
        // Make sure we don't hog the CPU: these loop { select! {} } blocks can starve the executor if we're not careful
        tokio::task::yield_now().await;

        // Draw the state of the server to the screen
        screen.render(&serve, &krate, &builder, &server, &watcher);

        // And then wait for any updates before redrawing
        let msg = tokio::select! {
            msg = watcher.wait(), if serve.should_hotreload() => msg,
            msg = server.wait() => msg,
            msg = builder.wait() => msg,
            msg = screen.wait() => match msg {
                Ok(res) => res,
                Err(_err) => break
            }
        };

        match msg {
            ServeUpdate::FilesChanged {} => {
                if !watcher.pending_changes() {
                    continue;
                }

                let changed_files = watcher.dequeue_changed_files(&krate);

                // if change is hotreloadable, hotreload it
                // and then send that update to all connected clients
                if let Some(hr) = watcher.attempt_hot_reload(&krate, changed_files) {
                    // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                    if hr.templates.is_empty() && hr.assets.is_empty() {
                        continue;
                    }

                    server.send_hotreload(hr).await;
                } else {
                    // If the change is not binary patchable, rebuild the project
                    // We're going to kick off a new build, interrupting the current build if it's ongoing
                    builder.build()?;

                    // Clear the hot reload changes
                    watcher.clear_hot_reload_changes();

                    // Tell the server to show a loading page for any new requests
                    server.start_build().await;
                }
            }

            // Run the server in the background
            // Waiting for updates here lets us tap into when clients are added/removed
            ServeUpdate::NewConnection => {
                if let Some(msg) = watcher.applied_hot_reload_changes() {
                    server.send_hotreload(msg).await;
                }
            }

            ServeUpdate::Message(msg) => {
                screen.new_ws_message(TargetPlatform::Web, msg);
            }

            // Wait for logs from the build engine
            // These will cause us to update the screen
            // We also can check the status of the builds here in case we have multiple ongoing builds
            ServeUpdate::Progress { update } => {
                let update_clone = update.clone();
                screen.new_build_logs(update.platform, update_clone);
                server
                    .update_build_status(screen.build_progress.progress(), update.stage.to_string())
                    .await;

                match update {
                    // Send rebuild start message.
                    UpdateBuildProgress {
                        stage: Stage::Compiling,
                        update: UpdateStage::Start,
                        platform: _,
                    } => server.send_reload_start().await,

                    // Send rebuild failed message.
                    UpdateBuildProgress {
                        stage: Stage::Finished,
                        update: UpdateStage::Failed(_),
                        platform: _,
                    } => server.send_reload_failed().await,

                    _ => {}
                }
            }

            ServeUpdate::BuildFailed { err, target } => {
                server.send_build_error(err).await;
            }

            ServeUpdate::BuildReady { target } => {
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

            ServeUpdate::TuiInput { rebuild } => {
                // Request a rebuild.
                if rebuild {
                    builder.build()?;
                    server.start_build().await
                }
            }
        }
    }

    // Kill the clients first
    _ = server.shutdown().await;
    _ = screen.shutdown();
    _ = builder.shutdown();

    Ok(())
}
