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
    std::env::var(APP_TITLE_ENV).ok()
}

pub fn always_on_top() -> Option<bool> {
    std::env::var(ALWAYS_ON_TOP_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn is_cli_enabled() -> bool {
    std::env::var(CLI_ENABLED_ENV).is_ok()
}

pub fn base_path() -> Option<PathBuf> {
    std::env::var("DIOXUS_ASSET_ROOT").ok().map(PathBuf::from)
}

pub fn out_dir() -> Option<PathBuf> {
    std::env::var(OUT_DIR).ok().map(PathBuf::from)
}
