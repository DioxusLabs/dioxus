pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PKG_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
pub const PKG_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
pub const PKG_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
pub const PKG_VERSION_PRE: &str = env!("CARGO_PKG_VERSION_PRE");

pub const GIT_COMMIT_HASH: Option<&str> = option_env!("DIOXUS_CLI_GIT_SHA");
pub const GIT_COMMIT_HASH_SHORT: Option<&str> = option_env!("DIOXUS_CLI_GIT_SHA_SHORT");
