use std::net::SocketAddr;

pub const DEVSERVER_RAW_ADDR_ENV: &str = "DIOXUS_DEVSERVER_ADDR";
pub const FULLSTACK_ADDRESS_ENV: &str = "DIOXUS_FULLSTACK_ADDRESS";

/// when targetting ios, we need to set a prefix to the argument such that it gets picked up by simctl
pub const IOS_DEVSERVER_ADDR_ENV: &str = "SIMCTL_CHILDDIOXUS_DEVSERVER_ADDR";

/// Get the address of the devserver for use over a raw socket
///
/// This is not a websocket! There's no protocol!
pub fn devserver_raw_addr() -> Option<String> {
    // #[cfg(target_os = "ios")]
    // return std::env::var(IOS_DEVSERVER_ADDR_ENV).ok();

    // #[cfg(not(target_os = "ios"))]
    return std::env::var(DEVSERVER_RAW_ADDR_ENV).ok();
}

pub fn fullstack_address() -> Option<SocketAddr> {
    std::env::var(FULLSTACK_ADDRESS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}
