use crate::{cfg::ConfigOptsServe, Result};
use dioxus_cli_config::CrateConfig;

mod build_engine;
mod connection;
mod fs_watcher;
mod tui_output;
mod web_server;
use build_engine::*;
use fs_watcher::*;
use tui_output::*;
use web_server::*;

/// For *all* builds the CLI spins up a dedicated webserver, file watcher, and build infrastructure to serve the project.
///
/// This includes web, desktop, mobile, fullstack, etc.
///
/// Platform specifics:
/// - Desktop:   We spin up the dev server but without a filesystem server.
/// - Mobile:    Basically the same as desktop.
/// - Web:       we need to attach a filesystem server to our devtools webserver to serve the project. We
///              want to emulate GithubPages here since most folks are deploying there and expect things like
///              basepath to match.
/// - Fullstack: We spin up the same dev server but in this case the fullstack server itself needs to
///              proxy all dev requests to our dev server
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
pub async fn dev_server(server_cfg: ConfigOptsServe, crate_config: CrateConfig) -> Result<()> {
    let mut file_watcher = FileWatcher::start(&crate_config);
    let mut web_server = WsServer::start(&server_cfg, &crate_config);
    let mut build_engine = BuildEngine::start();
    let mut screen = TuiOutput::start();

    loop {
        tokio::select! {
            // rebuild the project or hotreload it
            _ = file_watcher.wait() => {
                // if change is hotreloadable, hotreload it
                // and then send that update to all connected clients

                // If the change is not hotreloadable, attempt to binary patch it
                // If that change is successful, send that update to all connected clients

                // If the change is not binary patchable, rebuild the project
                // We're going to kick off a new build if it already isn't ongoing
            }

            // reload the page
            _ = web_server.wait() => {
                // Run the server in the background
                // Waiting for updates here lets us tap into when clients are added/removed
            }

            // Handle updates from the build engine
            _ = build_engine.wait() => {
                // Wait for logs from the build engine
                // These will cause us to update the screen
                // We also can check the status of the builds here in case we have multiple ongoing builds
            }

            // Handle input from the user using our settings
            input = screen.wait() => {
                // If the user wants to shutdown the server, break the loop
                if matches!(input, tui_output::TuiInput::Shutdown) {
                    break;
                }

                screen.handle_input(input);
            }
        }

        // Now that the updates are handled, draw the screen
        screen.draw(&server_cfg, &crate_config);
    }

    // Run our cleanup logic here - maybe printing as we go?
    // - Stop the file watcher
    // - tell our clients that we're shutting down (web pages)

    Ok(())
}
