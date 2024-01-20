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
