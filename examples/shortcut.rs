//! Add global shortcuts to your app while a component is active
//!
//! This demo shows how to add a global shortcut to your app that toggles a signal. You could use this to implement
//! a raycast-type app, or to add a global shortcut to your app that toggles a component on and off.
//!
//! These are *global* shortcuts, so they will work even if your app is not in focus.

use dioxus::desktop::use_global_shortcut;
use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut toggled = use_signal(|| false);

    _ = use_global_shortcut("ctrl+s", move || toggled.toggle());

    rsx!("toggle: {toggled}")
}
