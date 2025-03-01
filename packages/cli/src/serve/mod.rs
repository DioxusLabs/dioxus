use crate::{BuildUpdate, Builder, Error, Platform, Result, ServeArgs, TraceController, TraceSrc};

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
/// The "server" is special here since "fullstack" is functionally just an addition to the regular client
/// setup.
///
/// Todos(Jon):
/// - I'd love to be able to configure the CLI while it's running so we can change settings on the fly.
/// - I want us to be able to detect a `server_fn` in the project and then upgrade from a static server
///   to a dynamic one on the fly.
pub(crate) async fn serve_all(mut args: ServeArgs) -> Result<()> {
    // Redirect all logging the cli logger
    let mut tracer = TraceController::redirect(args.is_interactive_tty());

    // Load the krate and resolve the server args against it - this might log so do it after we turn on the tracer first
    let krate = args.load_krate().await?;

    // Note that starting the builder will queue up a build immediately
    let mut screen = Output::start(&args).await?;
    let mut builder = Builder::start(&krate, args.build_args())?;
    let mut devserver = WebServer::start(&krate, &args)?;
    let mut watcher = Watcher::start(&krate, &args);
    let mut runner = AppRunner::start(&krate);

    // This is our default splash screen. We might want to make this a fancier splash screen in the future
    // Also, these commands might not be the most important, but it's all we've got enabled right now
    tracing::info!(
        r#"-----------------------------------------------------------------
                Serving your Dioxus app: {} 🚀
                • Press `ctrl+c` to exit the server
                • Press `r` to rebuild the app
                • Press `p` to toggle automatic rebuilds
                • Press `v` to toggle verbose logging
                • Press `/` for more commands and shortcuts
                Learn more at https://dioxuslabs.com/learn/0.6/getting_started
               ----------------------------------------------------------------"#,
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
                match runner.hotreload(files).await {
                    HotReloadKind::Rsx(hr) => {
                        // Only send a hotreload message for templates and assets - otherwise we'll just get a full rebuild
                        //
                        // Also make sure the builder isn't busy since that might cause issues with hotreloads
                        // https://github.com/DioxusLabs/dioxus/issues/3361
                        if hr.is_empty() || !builder.can_receive_hotreloads() {
                            tracing::debug!(dx_src = ?TraceSrc::Dev, "Ignoring file change: {}", file);
                            continue;
                        }
                        tracing::info!(dx_src = ?TraceSrc::Dev, "Hotreloading: {}", file);
                        devserver.send_hotreload(hr).await;
                    }
                    HotReloadKind::Patch => {
                        if let Some(handle) = runner.running.as_ref() {
                            builder.patch_rebuild(
                                args.build_arguments.clone(),
                                vec![],
                                // handle.app.app.direct_rustc.last().unwrap().clone(),
                            );

                            runner.clear_hot_reload_changes();
                            runner.clear_cached_rsx();

                            devserver.start_patch().await
                        }
                    }
                    HotReloadKind::Full {} => todo!(),
                }
            }

            ServeUpdate::RequestRebuild => {
                // The spacing here is important-ish: we want
                // `Full rebuild:` to line up with
                // `Hotreloading:` to keep the alignment during long edit sessions
                tracing::info!("Full rebuild: triggered manually");

                builder.rebuild(args.build_arguments.clone());

                runner.clear_hot_reload_changes();
                runner.clear_cached_rsx();

                devserver.send_reload_start().await;
                devserver.start_build().await
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
                devserver.new_build_update(&update, &builder).await;

                // And then open the app if it's ready
                // todo: there might be more things to do here that require coordination with other pieces of the CLI
                // todo: maybe we want to shuffle the runner around to send an "open" command instead of doing that
                match update {
                    BuildUpdate::Progress { .. } => {}
                    BuildUpdate::CompilerMessage { message } => {
                        screen.push_cargo_log(message);
                    }
                    BuildUpdate::BuildFailed { err } => {
                        tracing::error!("Build failed: {:?}", err);
                    }

                    BuildUpdate::BuildReady { bundle } if bundle.build.is_patch() => {
                        let changed_symbols = runner.patch(&bundle).await?;
                        devserver
                            .send_patch(bundle.patch_exe(), changed_symbols)
                            .await;
                    }

                    BuildUpdate::BuildReady { bundle } => {
                        let handle = runner
                            .open(
                                bundle,
                                devserver.devserver_address(),
                                devserver.proxied_server_address(),
                                args.open.unwrap_or(false),
                            )
                            .await
                            .inspect_err(|e| tracing::error!("Failed to open app: {}", e));

                        // Update the screen + devserver with the new handle info
                        if handle.is_ok() {
                            devserver.send_reload_command().await
                        }
                    }
                }
            }

            // If the process exited *cleanly*, we can exit
            ServeUpdate::HandleUpdate(HandleUpdate::ProcessExited { status, platform }) => {
                if !status.success() {
                    tracing::error!("Application [{platform}] exited with error: {status}");
                } else {
                    tracing::info!(
                        r#"Application [{platform}] exited gracefully.
               - To restart the app, press `r` to rebuild or `o` to open
               - To exit the server, press `ctrl+c`"#
                    );
                }
            }

            ServeUpdate::HandleUpdate(HandleUpdate::StdoutReceived { platform, msg }) => {
                screen.push_stdio(platform, msg, tracing::Level::INFO);
            }

            ServeUpdate::HandleUpdate(HandleUpdate::StderrReceived { platform, msg }) => {
                screen.push_stdio(platform, msg, tracing::Level::ERROR);
            }

            ServeUpdate::TracingLog { log } => {
                screen.push_log(log);
            }

            ServeUpdate::OpenApp => {
                if let Err(err) = runner.open_existing(&devserver).await {
                    tracing::error!("Failed to open app: {err}")
                }
            }

            ServeUpdate::Redraw => {
                // simply returning will cause a redraw
            }

            ServeUpdate::ToggleShouldRebuild => {
                runner.automatic_rebuilds = !runner.automatic_rebuilds;
                tracing::info!(
                    "Automatic rebuilds are currently: {}",
                    if runner.automatic_rebuilds {
                        "enabled"
                    } else {
                        "disabled"
                    }
                )
            }

            ServeUpdate::Exit { error } => match error {
                Some(err) => break Err(anyhow::anyhow!("{}", err).into()),
                None => break Ok(()),
            },
        }
    };

    _ = runner.cleanup().await;
    _ = devserver.shutdown().await;
    builder.abort_all();
    _ = screen.shutdown();

    if let Err(err) = err {
        eprintln!("Exiting with error: {}", err);
    }

    Ok(())
}
