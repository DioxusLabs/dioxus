static mut BUNDLED: bool = false;
static mut CHECKED_YET: bool = false;

pub fn is_bundled_app() -> bool {
    // unsafe to implement a dumb cell without pulling in deps / mutexes
    // we're okay with a race condition here since the bundled flag is always set to the same value
    unsafe {
        if !CHECKED_YET {
            CHECKED_YET = true;
            BUNDLED = std::env::var("DIOXUS_CLI_ENABLED").is_ok()
                || !matches!(std::env::var("CARGO_MANIFEST_DIR"), Ok(path) if !path.is_empty());
        }

        BUNDLED
    }
}
