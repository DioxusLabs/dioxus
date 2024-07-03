use crate::{cfg::ConfigOptsServe, Result};
use dioxus_cli_config::CrateConfig;
use tokio::task::yield_now;

mod builder;
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
pub async fn serve_all(srv: ConfigOptsServe, crt: CrateConfig) -> Result<()> {
    let mut server = Server::start(&srv, &crt).await;
    let mut watchr = Watcher::start(&crt);
    let mut screen = Output::start(&srv, &crt);
    let mut buildr = Builder::start(&srv, &crt);

    loop {
        // Make sure we don't hog the CPU: these loop { select! {} } blocks can starve the executor
        yield_now().await;

        // Draw the state of the server to the screen
        screen.draw(&srv, &crt, &buildr, &server, &watchr);

        // Also update the webserver page if we need to
        // This will send updates about the current status of the build
        server.update(&srv, &crt);

        // And then wait for any updates before redrawing
        tokio::select! {
            // rebuild the project or hotreload it
            _ = watchr.wait() => {
                if !watchr.pending_changes() {
                    continue;
                }

                // if change is hotreloadable, hotreload it
                // and then send that update to all connected clients
                if let Some(hr) = watchr.attempt_hot_reload(&crt) {
                    server.send_hotreload(hr).await;
                    continue;
                }

                // If the change is not binary patchable, rebuild the project
                // We're going to kick off a new build if it already isn't ongoing
                // todo: we need to know which project to build here - either the frontend or the backend
                buildr.queue_build();
            }

            // reload the page
            new_socket = server.wait() => {
                // Run the server in the background
                // Waiting for updates here lets us tap into when clients are added/removed
            }

            // Handle updates from the build engine
            _ = buildr.wait() => {
                // Wait for logs from the build engine
                // These will cause us to update the screen
                // We also can check the status of the builds here in case we have multiple ongoing builds
            }

            // Handle input from the user using our settings
            input = screen.wait() => {
                // If the user wants to shutdown the server, break the loop
                if matches!(input, output::TuiInput::Shutdown) {
                    break;
                }

                // Maybe handle manually reloading?
                screen.handle_input(input);
            }
        }
    }

    // Run our cleanup logic here - maybe printing as we go?
    // tood: more printing, logging, error handling in this phase
    _ = server.shutdown().await;
    _ = buildr.shutdown().await;

    Ok(())
}
