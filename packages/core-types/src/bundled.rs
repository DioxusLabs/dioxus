use once_cell::sync::Lazy;

pub fn is_bundled_app() -> bool {
    static BUNDLED: Lazy<bool> = Lazy::new(|| {
        // If the env var is set, we're bundled
        if std::env::var("DIOXUS_CLI_ENABLED").is_ok() {
            return true;
        }

        // If the cargo manifest dir is set, we're not bundled.
        // Generally this is only set with `cargo run`
        if let Ok(path) = std::env::var("CARGO_MANIFEST_DIR") {
            if !path.is_empty() {
                return false;
            }
        }

        true
    });

    *BUNDLED
}
