use crate::{
    BuildUpdate, Builder, DioxusCrate, Error, Platform, Result, ServeArgs, TraceController,
    TraceSrc,
};

mod ansi_buffer;
mod detect;
mod handle;
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
    let mut devserver = WebServer::start(&args, &krate)?;
    let mut screen = Output::start(&args).expect("Failed to open terminal logger");
    let mut watcher = Watcher::start(&args, &krate);
    let mut runner = AppRunner::start(&krate);

    // This is our default splash screen. We might want to make this a fancier splash screen in the future
    // Also, these commands might not be the most important, but it's all we've got enabled right now
    tracing::info!(
        r#"Serving your Dioxus app: {} 🚀

                - Press `ctrl+c` to exit the server
                - Press `r` to rebuild the app
                - Press `o` to open the app
                - Press `/` for more commands and shortcuts

                To learn more, check out the docs at https://dioxuslabs.com/learn/0.6/getting_started
    "#,
        krate.executable_name()
    );

    let err: Result<(), Error> = loop {
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
                if let Some(hr) = runner.attempt_hot_reload(files) {
                    // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                    if hr.templates.is_empty()
                        && hr.assets.is_empty()
                        && hr.unknown_files.is_empty()
                    {
                        tracing::info!(dx_src = ?TraceSrc::Dev, "Ignoring file change: {}", file);
                        continue;
                    }

                    tracing::info!(dx_src = ?TraceSrc::Dev, "Hotreloading: {}", file);

                    devserver.send_hotreload(hr).await;
                } else {
                    tracing::info!(dx_src = ?TraceSrc::Dev, "Full rebuild: {}", file);

                    // We're going to kick off a new build, interrupting the current build if it's ongoing
                    builder.rebuild(args.build_arguments.clone());

                    // Clear the hot reload changes so we don't have out-of-sync issues with changed UI
                    runner.clear_hot_reload_changes();

                    // Tell the server to show a loading page for any new requests
                    devserver.start_build().await;
                }
            }

            // Run the server in the background
            // Waiting for updates here lets us tap into when clients are added/removed
            ServeUpdate::NewConnection => {
                devserver
                    .send_hotreload(runner.applied_hot_reload_changes())
                    .await;

                runner.client_connected().await;
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
                    BuildUpdate::CompilerMessage { message } => {
                        screen.push_cargo_log(message);
                    }
                    BuildUpdate::BuildFailed { err } => {
                        tracing::error!("Build failed: {}", err);
                    }
                    BuildUpdate::BuildReady { bundle } => {
                        let handle = runner
                            .open(
                                bundle,
                                devserver.devserver_address(),
                                devserver.proxied_server_address(),
                                args.open.unwrap_or(false),
                            )
                            .await;

                        match handle {
                            // Update the screen + devserver with the new handle info
                            Ok(_handle) => {
                                devserver.send_reload_command().await;
                            }

                            Err(e) => tracing::error!("Failed to open app: {}", e),
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

                runner.kill(platform);
            }

            ServeUpdate::StdoutReceived { platform, msg } => {
                screen.push_stdio(platform, msg, tracing::Level::INFO);
            }

            ServeUpdate::StderrReceived { platform, msg } => {
                screen.push_stdio(platform, msg, tracing::Level::ERROR);
            }

            ServeUpdate::TracingLog { log } => {
                screen.push_log(log);
            }

            ServeUpdate::RequestRebuild => {
                // The spacing here is important-ish: we want
                // `Full rebuild:` to line up with
                // `Hotreloading:` to keep the alignment during long edit sessions
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
