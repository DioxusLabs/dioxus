//! <div align="center">
//!     <img
//!         src="https://github.com/user-attachments/assets/6c7e227e-44ff-4e53-824a-67949051149c"
//!         alt="Build web, desktop, and mobile apps with a single codebase."
//!         width="100%"
//!         class="darkmode-image"
//!     >
//! </div>
//!
//! # Dioxus CLI configuration
//!
//! This crate exposes the various configuration options that the Dioxus CLI sets when running the
//! application during development.
//!
//! Note that these functions will return a different value when running under the CLI, so make sure
//! not to rely on them when running in a production environment.
//!
//! ## Constants
//!
//! The various constants here are the names of the environment variables that the CLI sets. We recommend
//! using the functions in this crate to access the values of these environment variables indirectly.
//!
//! The CLI uses this crate and the associated constants to *set* the environment variables, but as
//! a consumer of the CLI, you would want to read the values of these environment variables using
//! the provided functions.
//!
//! ## Example Usage
//!
//! We recommend using the functions here to access the values of the environment variables set by the CLI.
//! For example, you might use the [`fullstack_address_or_localhost`] function to get the address that
//! the CLI is requesting the application to be served on.
//!
//! ```rust, ignore
//! async fn launch_axum(app: axum::Router<()>) {
//!     // Read the PORT and ADDR environment variables set by the CLI
//!     let addr = dioxus_cli_config::fullstack_address_or_localhost();
//!
//!     // Bind to the address and serve the application
//!     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
//!     axum::serve(listener, app.into_make_service())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! ## Stability
//!
//! The *values* that these functions return are *not* guaranteed to be stable between patch releases
//! of Dioxus. At any time, we might change the values that the CLI sets or the way that they are read.
//!
//! We also don't guarantee the stability of the env var names themselves. If you want to rely on a
//! particular env var, use the defined constants in your code.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

pub const CLI_ENABLED_ENV: &str = "DIOXUS_CLI_ENABLED";
pub const SERVER_IP_ENV: &str = "IP";
pub const SERVER_PORT_ENV: &str = "PORT";
pub const DEVSERVER_RAW_ADDR_ENV: &str = "DIOXUS_DEVSERVER_ADDR";
pub const ALWAYS_ON_TOP_ENV: &str = "DIOXUS_ALWAYS_ON_TOP";
pub const ASSET_ROOT_ENV: &str = "DIOXUS_ASSET_ROOT";
pub const APP_TITLE_ENV: &str = "DIOXUS_APP_TITLE";

#[deprecated(since = "0.6.0", note = "The CLI currently does not set this.")]
#[doc(hidden)]
pub const OUT_DIR: &str = "DIOXUS_OUT_DIR";
pub const SESSION_CACHE_DIR: &str = "DIOXUS_SESSION_CACHE_DIR";

/// Reads an environment variable at runtime in debug mode or at compile time in
/// release mode. When bundling in release mode, we will not be running under the
/// environment variables that the CLI sets, so we need to read them at compile time.
macro_rules! read_env_config {
    ($name:expr) => {{
        #[cfg(debug_assertions)]
        {
            // In debug mode, read the environment variable set by the CLI at runtime
            std::env::var($name).ok()
        }

        #[cfg(not(debug_assertions))]
        {
            // In release mode, read the environment variable set by the CLI at compile time
            // This means the value will still be available when running the application
            // standalone.
            // We don't always read the environment variable at compile time to avoid rebuilding
            // this crate when the environment variable changes.
            option_env!($name).map(ToString::to_string)
        }
    }};
}

/// Get the address of the devserver for use over a raw socket
///
/// This returns a [`SocketAddr`], meaning that you still need to connect to it using a socket with
/// the appropriate protocol and path.
///
/// For reference, the devserver typically lives on `127.0.0.1:8080` and serves the devserver websocket
/// on `127.0.0.1:8080/_dioxus`.
pub fn devserver_raw_addr() -> Option<SocketAddr> {
    let addr = std::env::var(DEVSERVER_RAW_ADDR_ENV).ok()?;
    addr.parse().ok()
}

/// Get the address of the devserver for use over a websocket
///
/// This is meant for internal use, though if you are building devtools around Dioxus, this would be
/// useful to connect as a "listener" to the devserver.
///
/// Unlike [`devserver_raw_addr`], this returns a string that can be used directly to connect to the
/// devserver over a websocket. IE `ws://127.0.0.1:8080/_dioxus`.
pub fn devserver_ws_endpoint() -> Option<String> {
    let addr = devserver_raw_addr()?;
    Some(format!("ws://{addr}/_dioxus"))
}

/// Get the IP that the server should be bound to.
///
/// This is set by the CLI and is used to bind the server to a specific address.
/// You can manually set the ip by setting the `IP` environment variable.
///
/// ```sh
/// IP=0.0.0.0 ./server
/// ```
pub fn server_ip() -> Option<IpAddr> {
    std::env::var(SERVER_IP_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Get the port that the server should listen on.
///
/// This is set by the CLI and is used to bind the server to a specific port.
/// You can manually set the port by setting the `PORT` environment variable.
///
/// ```sh
/// PORT=8081 ./server
/// ```
pub fn server_port() -> Option<u16> {
    std::env::var(SERVER_PORT_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Get the full address that the server should listen on.
///
/// This is a convenience function that combines the `server_ip` and `server_port` functions and then
/// falls back to `localhost:8080` if the environment variables are not set.
///
/// ## Example
///
/// ```rust, ignore
/// async fn launch_axum(app: axum::Router<()>) {
///     // Read the PORT and ADDR environment variables set by the CLI
///     let addr = dioxus_cli_config::fullstack_address_or_localhost();
///
///     // Bind to the address and serve the application
///     let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
///     axum::serve(listener, app.into_make_service())
///         .await
///         .unwrap();
/// }
/// ```
///
/// ## Stability
///
/// In the future, we might change the address from 127.0.0.1 to 0.0.0.0.
pub fn fullstack_address_or_localhost() -> SocketAddr {
    let ip = server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = server_port().unwrap_or(8080);
    SocketAddr::new(ip, port)
}

/// Get the title of the application, usually set by the Dioxus.toml.
///
/// This is used to set the title of the desktop window if the app itself doesn't set it.
pub fn app_title() -> Option<String> {
    read_env_config!("DIOXUS_APP_TITLE")
}

/// Check if the application should forced to "float" on top of other windows.
///
/// The CLI sets this based on the `--always-on-top` flag and the settings system.
pub fn always_on_top() -> Option<bool> {
    std::env::var(ALWAYS_ON_TOP_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Check if the CLI is enabled when running the application.
///
/// The CLI *always* sets this value to true when running the application.
///
/// ## Note
///
/// On Android and the Web, this *might* not be reliable since there isn't always a consistent way to
/// pass off the CLI environment variables to the application.
pub fn is_cli_enabled() -> bool {
    // todo: (jon) - on android and web we should fix this...
    std::env::var(CLI_ENABLED_ENV).is_ok()
}

/// Get the path where the application will be served from.
///
/// This is used by the router to format the URLs. For example, an app with a base path of `dogapp` will
/// be served at `http://localhost:8080/dogapp`.
///
/// All assets will be served from this base path as well, ie `http://localhost:8080/dogapp/assets/logo.png`.
pub fn base_path() -> Option<String> {
    // This may trigger when compiling to the server if you depend on another crate that pulls in
    // the web feature. It might be better for the renderers to provide the current platform
    // as a global context
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        return web_base_path();
    }

    read_env_config!("DIOXUS_ASSET_ROOT")
}

#[cfg(feature = "web")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
        export function getMetaContents(meta_name) {
            const selector = document.querySelector(`meta[name="${meta_name}"]`);
            if (!selector) {
                return null;
            }
            return selector.content;
        }
    "#)]
extern "C" {
    #[wasm_bindgen(js_name = getMetaContents)]
    pub fn get_meta_contents(selector: &str) -> Option<String>;
}

/// Get the path where the application is served from in the browser.
///
/// This uses wasm_bindgen on the browser to extract the base path from a meta element.
#[cfg(feature = "web")]
pub fn web_base_path() -> Option<String> {
    // In debug mode, we get the base path from the meta element which can be hot reloaded and changed without recompiling
    #[cfg(debug_assertions)]
    {
        thread_local! {
            static BASE_PATH: std::cell::OnceCell<Option<String>> = const { std::cell::OnceCell::new() };
        }
        BASE_PATH.with(|f| f.get_or_init(|| get_meta_contents(ASSET_ROOT_ENV)).clone())
    }

    // In release mode, we get the base path from the environment variable
    #[cfg(not(debug_assertions))]
    {
        option_env!("DIOXUS_ASSET_ROOT").map(ToString::to_string)
    }
}

/// Format a meta element for the base path to be used in the output HTML
#[doc(hidden)]
pub fn format_base_path_meta_element(base_path: &str) -> String {
    format!(r#"<meta name="{ASSET_ROOT_ENV}" content="{base_path}">"#,)
}

/// Get the path to the output directory where the application is being built.
///
/// This might not return a valid path - we don't recommend relying on this.
#[doc(hidden)]
#[deprecated(
    since = "0.6.0",
    note = "The does not set the OUT_DIR environment variable."
)]
pub fn out_dir() -> Option<PathBuf> {
    #[allow(deprecated)]
    {
        std::env::var(OUT_DIR).ok().map(PathBuf::from)
    }
}

/// Get the directory where this app can write to for this session that's guaranteed to be stable
/// between reloads of the same app. This is useful for emitting state like window position and size
/// so the app can restore it when it's next opened.
///
/// Note that this cache dir is really only useful for platforms that can access it. Web/Android
/// don't have access to this directory, so it's not useful for them.
///
/// This is designed with desktop executables in mind.
pub fn session_cache_dir() -> Option<PathBuf> {
    if cfg!(target_os = "android") {
        return Some(android_session_cache_dir());
    }

    std::env::var(SESSION_CACHE_DIR).ok().map(PathBuf::from)
}

/// The session cache directory for android
pub fn android_session_cache_dir() -> PathBuf {
    PathBuf::from("/data/local/tmp/dx/")
}
