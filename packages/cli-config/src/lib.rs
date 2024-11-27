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
pub const OUT_DIR: &str = "DIOXUS_OUT_DIR";

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
/// This is not a websocket! There's no protocol!
pub fn devserver_raw_addr() -> Option<SocketAddr> {
    // On android, 10.0.2.2 is the default loopback
    if cfg!(target_os = "android") {
        return Some("10.0.2.2:8080".parse().unwrap());
    }

    std::env::var(DEVSERVER_RAW_ADDR_ENV)
        .map(|s| s.parse().ok())
        .ok()
        .flatten()
}

pub fn devserver_ws_endpoint() -> Option<String> {
    let addr = devserver_raw_addr()?;
    Some(format!("ws://{addr}/_dioxus"))
}

pub fn server_ip() -> Option<IpAddr> {
    std::env::var(SERVER_IP_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn server_port() -> Option<u16> {
    std::env::var(SERVER_PORT_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn fullstack_address_or_localhost() -> SocketAddr {
    let ip = server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = server_port().unwrap_or(8080);
    SocketAddr::new(ip, port)
}

pub fn app_title() -> Option<String> {
    read_env_config!("DIOXUS_APP_TITLE")
}

pub fn always_on_top() -> Option<bool> {
    std::env::var(ALWAYS_ON_TOP_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn is_cli_enabled() -> bool {
    std::env::var(CLI_ENABLED_ENV).is_ok()
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

/// Get the path where the application will be served from. This is used by the router to format the URLs.
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

/// Get the path where the application is served from in the browser.
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

pub fn format_base_path_meta_element(base_path: &str) -> String {
    format!(r#"<meta name="{ASSET_ROOT_ENV}" content="{base_path}">"#,)
}

pub fn out_dir() -> Option<PathBuf> {
    std::env::var(OUT_DIR).ok().map(PathBuf::from)
}
