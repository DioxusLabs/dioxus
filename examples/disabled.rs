use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let disabled = use_signal(|| false);

    rsx! {
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
