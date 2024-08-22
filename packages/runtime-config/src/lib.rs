use std::net::SocketAddr;

pub const DEVSERVER_ADDR_ENV: &str = "DIOXUS_DEVSERVER_ADDR";

/// Get the address of the devserver
pub fn devserver_addr() -> Option<SocketAddr> {
    std::env::var(DEVSERVER_ADDR_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub const FULLSTACK_ADDRESS_ENV: &str = "DIOXUS_FULLSTACK_ADDRESS";

pub fn fullstack_address() -> Option<SocketAddr> {
    std::env::var(FULLSTACK_ADDRESS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}
