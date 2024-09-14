use crate::builder::{BuildUpdate, BuildUpdateProgress, Builder, Platform, Stage, UpdateStage};
use crate::cli::serve::ServeArgs;
use crate::DioxusCrate;
use crate::Result;
use crate::TraceSrc;
use std::ops::ControlFlow;

mod console_widget;
mod detect;
mod handle;
mod hot_reloading_file_map;
mod logs_tab;
mod output;
mod proxy;
mod runner;
mod server;
mod tracer;
mod update;
mod watcher;

pub(crate) use handle::*;
pub(crate) use output::*;
pub(crate) use runner::*;
pub(crate) use server::*;
pub(crate) use tracer::*;
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
/// - Handle logs from the build engine separately?
/// - I want us to be able to detect a `server_fn` in the project and then upgrade from a static server
///   to a dynamic one on the fly.
pub(crate) async fn serve_all(args: ServeArgs, krate: DioxusCrate) -> Result<()> {
    let mut tracer = tracer::TraceController::start();

    // Note that starting the builder will queue up a build immediately
    let mut builder = Builder::start(&krate, args.build_args())?;
    let mut screen = Output::start(&args).expect("Failed to open terminal logger");
    let mut devserver = DevServer::start(&args, &krate);
    let mut watcher = Watcher::start(&args, &krate);
    let mut runner = AppRunner::start();

    loop {
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

        let res = handle_update(
            msg,
            &args,
            &mut devserver,
            &mut screen,
            &mut builder,
            &mut runner,
            &mut watcher,
        );

        match res.await {
            Ok(ControlFlow::Continue(())) => continue,
            Ok(ControlFlow::Break(())) => {}
            Err(e) => tracing::error!("Error in TUI: {}", e),
        }

        break;
    }

    _ = devserver.shutdown().await;
    _ = screen.shutdown();
    _ = builder.abort_all();
    _ = tracer.shutdown();

    Ok(())
}

async fn handle_update(
    msg: ServeUpdate,
    args: &ServeArgs,
    devserver: &mut DevServer,
    screen: &mut Output,
    builder: &mut Builder,
    runner: &mut AppRunner,
    watcher: &mut Watcher,
) -> Result<ControlFlow<()>> {
    match msg {
        ServeUpdate::FilesChanged { files } => {
            if files.is_empty() || !args.should_hotreload() {
                return Ok(ControlFlow::Continue(()));
            }

            // if change is hotreloadable, hotreload it
            // and then send that update to all connected clients
            if let Some(hr) = watcher.attempt_hot_reload(files, &runner) {
                // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                if hr.templates.is_empty() && hr.assets.is_empty() {
                    return Ok(ControlFlow::Continue(()));
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
                BuildUpdateProgress {
                    stage: Stage::Compiling,
                    update: UpdateStage::Start,
                    platform: _,
                } => devserver.send_reload_start().await,

                // Send rebuild failed message.
                BuildUpdateProgress {
                    stage: Stage::Finished,
                    update: UpdateStage::Failed(_),
                    platform: _,
                } => devserver.send_reload_failed().await,

                _ => {}
            }
        }

        ServeUpdate::BuildUpdate(BuildUpdate::BuildFailed { err, .. }) => {
            devserver.send_build_error(err).await;
        }

        ServeUpdate::BuildUpdate(BuildUpdate::BuildReady { target, result }) => {
            tracing::info!(dx_src = ?TraceSrc::Dev, "Opening app for [{}]", target);

            let handle = runner
                .open(result, devserver.ip, devserver.fullstack_address())
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

        // nothing - the builder just signals that there are no more pending builds
        ServeUpdate::BuildUpdate(BuildUpdate::AllFinished) => {}

        // If the process exited *cleanly*, we can exit
        ServeUpdate::ProcessExited { status, platform } => {
            if !status.success() {
                tracing::error!(dx_src = ?TraceSrc::Dev, "Application [{platform}] exited with status: {status}");
                return Ok(ControlFlow::Break(()));
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

        ServeUpdate::TuiInput { event } => {
            let should_rebuild = screen.handle_input(event)?;
            if should_rebuild {
                builder.build(args.build_arguments.clone())?;
                devserver.start_build().await
            }
        }
    }

    Ok(ControlFlow::Continue(()))
}
