use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut disabled = use_signal(|| false);

    render! {
        div {
            button { onclick: move |_| disabled.toggle(),
                "click to "
                if disabled() { "enable" } else { "disable" }
                " the lower button"
            }

            button { disabled, "lower button" }
        }
    }
}
