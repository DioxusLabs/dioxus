use dioxus::prelude::*;
use dioxus_desktop::use_global_shortcut;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut toggled = use_signal(|| false);

    _ = use_global_shortcut("ctrl+s", move || toggled.toggle());

    render!("toggle: {toggled}")
}
