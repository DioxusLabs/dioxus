use dioxus::{core::CapturedError, prelude::*};

fn main() {
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        ErrorBoundary {
            handle_error: |error: CapturedError| rsx! {"Found error {error}"},
            DemoC {
                x: 1
            }
        }
    })
}

#[component]
fn DemoC(cx: Scope, x: i32) -> Element {
    let result = Err("Error");

    result.throw()?;

    render! {
        h1 { "{x}" }
    }
}
