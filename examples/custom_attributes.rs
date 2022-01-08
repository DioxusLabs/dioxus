//! Use custom formatting in attributes to set attributes

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let disabled = use_state(&cx, || false);

    let button_disabled = if *disabled { "disabled" } else { "nnn" };

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| disabled.set(!disabled.get()),
                "click to " [if *disabled {"enable"} else {"disable"} ] " the lower button"
            }

            button {
                "{button_disabled}": "true",
                "lower button"
            }
        }
    })
}
