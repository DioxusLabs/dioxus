use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let disabled = use_state(&cx, || false);

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| disabled.set(!disabled.get()),
                "click to " [if **disabled {"enable"} else {"disable"} ] " the lower button"
            }

            button {
                disabled: "{disabled}",
                "lower button"
            }

            input {
                value: "false",
            }

        }
    })
}
