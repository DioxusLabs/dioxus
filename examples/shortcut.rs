use dioxus::prelude::*;
use dioxus_desktop::use_global_shortcut;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let toggled = use_state(|| false);

    use_global_shortcut("ctrl+s", {
        to_owned![toggled];
        move || toggled.modify(|t| !*t)
    });

    cx.render(rsx!("toggle: {toggled.get()}"))
}
