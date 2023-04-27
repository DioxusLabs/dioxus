// Check that all events are being forwarded to the mock.
//! This example roughly shows how events are serialized into Rust from JavaScript.
//!
//! There is some conversion happening when input types are checkbox/radio/select/textarea etc.

use dioxus::prelude::*;

mod events;

fn main() {
    events::test_events();
}

pub(crate) fn check_app_exits(app: Component) {
    // This is a deadman's switch to ensure that the app exits
    let should_panic = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let should_panic_clone = should_panic.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(100));
        if should_panic_clone.load(std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(exitcode::SOFTWARE);
        }
    });

    dioxus_desktop::launch(app);

    should_panic.store(false, std::sync::atomic::Ordering::SeqCst);
}
