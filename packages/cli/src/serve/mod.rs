use std::future::{poll_fn, Future, IntoFuture};
use std::task::Poll;

use crate::builder::{Stage, TargetPlatform, UpdateBuildProgress, UpdateStage};
use crate::cli::serve::Serve;
use crate::dioxus_crate::DioxusCrate;
use crate::tracer::CLILogControl;
use crate::Result;
use futures_util::FutureExt;
use tokio::task::yield_now;

mod builder;
mod hot_reloading_file_map;
mod logs_tab;
mod output;
mod proxy;
mod server;
mod watcher;

use builder::*;
use output::*;
use server::*;
use watcher::*;

/// For *all* builds the CLI spins up a dedicated webserver, file watcher, and build infrastructure to serve the project.
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
/// - I'd love to be able to configure the CLI while it's running so we can change settingaon the fly.
///   This would require some light refactoring and potentially pulling in something like ratatui.
/// - Build a custom subscriber for logs by tools within this
/// - Handle logs from the build engine separately?
/// - Consume logs from the wasm for web/fullstack
/// - I want us to be able to detect a `server_fn` in the project and then upgrade from a static server
///   to a dynamic one on the fly.
pub async fn serve_all(
    serve: Serve,
    dioxus_crate: DioxusCrate,
    log_control: CLILogControl,
) -> Result<()> {
    let mut builder = Builder::new(&dioxus_crate, &serve);

    // Start the first build
    builder.build();

    let mut server = Server::start(&serve, &dioxus_crate);
    let mut watcher = Watcher::start(&serve, &dioxus_crate);
    let mut screen = Output::start(&serve, log_control).expect("Failed to open terminal logger");

    let is_hot_reload = serve.server_arguments.hot_reload.unwrap_or(true);

    loop {
        // Make sure we don't hog the CPU: these loop { select! {} } blocks can starve the executor
        yield_now().await;

        // Draw the state of the server to the screen
        screen.render(&serve, &dioxus_crate, &builder, &server, &watcher);

        // And then wait for any updates before redrawing
        tokio::select! {
            // rebuild the project or hotreload it
            _ = watcher.wait(), if is_hot_reload => {
                if !watcher.pending_changes() {
                    continue
                }

                let changed_files = watcher.dequeue_changed_files(&dioxus_crate);

                // if change is hotreloadable, hotreload it
                // and then send that update to all connected clients
                if let Some(hr) = watcher.attempt_hot_reload(&dioxus_crate, changed_files) {
                    // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                    if hr.templates.is_empty() && hr.assets.is_empty() {
                        continue
                    }

                    server.send_hotreload(hr).await;
                } else {
                    // If the change is not binary patchable, rebuild the project
                    // We're going to kick off a new build, interrupting the current build if it's ongoing
                    builder.build();

                    // Tell the server to show a loading page for any new requests
                    server.start_build().await;
                }
            }

            // reload the page
            msg = server.wait() => {
                // Run the server in the background
                // Waiting for updates here lets us tap into when clients are added/removed
                if let Some(msg) = msg {
                    screen.new_ws_message(TargetPlatform::Web, msg);
                }
            }

            // Handle updates from the build engine
            application = builder.wait() => {
                // Wait for logs from the build engine
                // These will cause us to update the screen
                // We also can check the status of the builds here in case we have multiple ongoing builds
                match application {
                    Ok(BuilderUpdate::Progress { platform, update }) => {
                        let update_clone = update.clone();
                        screen.new_build_logs(platform, update_clone);
                        server.update_build_status(screen.build_progress.progress(), update.stage.to_string()).await;

                        match update {
                            // Send rebuild start message.
                            UpdateBuildProgress { stage: Stage::Compiling, update: UpdateStage::Start } => server.send_reload_start().await,
                            // Send rebuild failed message.
                            UpdateBuildProgress { stage: Stage::Finished, update: UpdateStage::Failed(_) } => server.send_reload_failed().await,
                            _ => {},
                        }
                    }
                    Ok(BuilderUpdate::Ready { results }) => {
                        if !results.is_empty() {
                            builder.children.clear();
                        }

                        // If we have a build result, open it
                        for build_result in results.iter() {
                            let child = build_result.open(&serve.server_arguments, server.fullstack_address(), &dioxus_crate.workspace_dir());
                            match child {
                                Ok(Some(child_proc)) => builder.children.push((build_result.target_platform, child_proc)),
                                Err(e) => {
                                    tracing::error!("Failed to open build result: {e}");
                                    break;
                                },
                                _ => {}
                            }
                        }

                        // Make sure we immediately capture the stdout/stderr of the executable -
                        // otherwise it'll clobber our terminal output
                        screen.new_ready_app(&mut builder, results);

                        // And then finally tell the server to reload
                        server.send_reload_command().await;
                    },

                    // If the process exited *cleanly*, we can exit
                    Ok(BuilderUpdate::ProcessExited { status, target_platform }) => {
                        // Then remove the child process
                        builder.children.retain(|(platform, _)| *platform != target_platform);
                        match status {
                            Ok(status) => {
                                if status.success() {
                                    break;
                                }
                                else {
                                    tracing::error!("Application exited with status: {status}");
                                }
                            },
                            Err(e) => {
                                tracing::error!("Application exited with error: {e}");
                            }
                        }
                    }
                    Err(err) => {
                        server.send_build_error(err).await;
                    }
                }
            }

            // Handle input from the user using our settings
            res = screen.wait() => {
                match res {
                    Ok(false) => {}
                    // Request a rebuild.
                    Ok(true) => {
                        builder.build();
                        server.start_build().await
                    },
                    // Shutdown the server.
                    Err(_) => break,
                }
            }
        }
    }

    // Run our cleanup logic here - maybe printing as we go?
    // todo: more printing, logging, error handling in this phase
    _ = screen.shutdown();
    _ = server.shutdown().await;
    builder.shutdown();

    Ok(())
}

// Grab the output of a future that returns an option or wait forever
pub(crate) fn next_or_pending<F, T>(f: F) -> impl Future<Output = T>
where
    F: IntoFuture<Output = Option<T>>,
{
    let pinned = f.into_future().fuse();
    let mut pinned = Box::pin(pinned);
    poll_fn(move |cx| {
        let next = pinned.as_mut().poll(cx);
        match next {
            Poll::Ready(Some(next)) => Poll::Ready(next),
            _ => Poll::Pending,
        }
    })
    .fuse()
}
