#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

struct DarkMode(bool);

#[rustfmt::skip]
pub fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || DarkMode(false));

    render!(
        DarkModeToggle {},
        AppBody {}
    )
}

pub fn DarkModeToggle(cx: Scope) -> Element {
    let dark_mode = use_shared_state::<DarkMode>(cx).unwrap();

    let style = if dark_mode.read().0 {
        "color:white"
    } else {
        ""
    };

    cx.render(rsx!(label {
        style: "{style}",
        "Dark Mode",
        input {
            r#type: "checkbox",
            oninput: move |event| {
                let is_enabled = event.value == "true";
                dark_mode.write().0 = is_enabled;
            },
        },
    }))
}

fn AppBody(cx: Scope) -> Element {
    let dark_mode = use_shared_state::<DarkMode>(cx).unwrap();

    let is_dark_mode = dark_mode.read().0;
    let answer = if is_dark_mode { "Yes" } else { "No" };

    render!(
        p {
            "Is Dark mode enabled? {answer}"
        }
    )
}
