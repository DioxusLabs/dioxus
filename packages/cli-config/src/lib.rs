use std::net::{IpAddr, SocketAddr};

pub const DEVSERVER_RAW_ADDR_ENV: &str = "DIOXUS_DEVSERVER_ADDR";
pub const FULLSTACK_IP: &str = "DIOXUS_FULLSTACK_IP";
pub const FULLSTACK_PORT_ENV: &str = "DIOXUS_FULLSTACK_PORT";

/// when targetting ios, we need to set a prefix to the argument such that it gets picked up by simctl
pub const IOS_DEVSERVER_ADDR_ENV: &str = "SIMCTL_CHILD_DIOXUS_DEVSERVER_ADDR";

/// Get the address of the devserver for use over a raw socket
///
/// This is not a websocket! There's no protocol!
pub fn devserver_raw_addr() -> Option<SocketAddr> {
    std::env::var(DEVSERVER_RAW_ADDR_ENV)
        .map(|s| s.parse().ok())
        .ok()
        .flatten()
}

pub fn devserver_ws_endpoint() -> Option<String> {
    let addr = devserver_raw_addr()?;
    Some(format!("ws://{addr}/_dioxus"))
}

pub fn fullstack_ip() -> Option<IpAddr> {
    std::env::var(FULLSTACK_IP)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn fullstack_port() -> Option<u16> {
    std::env::var(FULLSTACK_PORT_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn fullstack_address() -> Option<SocketAddr> {
    let ip = fullstack_ip()?;
    let port = fullstack_port()?;
    Some(SocketAddr::new(ip, port))
}
