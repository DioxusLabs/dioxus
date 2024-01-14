use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let disabled = use_state(|| false);

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| disabled.set(!disabled),
                "click to "
                if disabled == true { "enable" } else { "disable" }
                " the lower button"
            }

            button {
                disabled: "{disabled}",
                "lower button"
            }
        }
    })
}
