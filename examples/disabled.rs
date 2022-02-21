use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let (disabled, set_disabled) = use_state(&cx, || false);

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| set_disabled(!disabled),
                "click to " [if *disabled {"enable"} else {"disable"} ] " the lower button"
            }

            button {
                disabled: "{disabled}",
                "lower button"
            }
        }
    })
}
