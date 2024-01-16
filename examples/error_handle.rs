use dioxus::{dioxus_core::CapturedError, prelude::*};

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    rsx! {
        ErrorBoundary {
            handle_error: |error: CapturedError| rsx! {"Found error {error}"},
            DemoC { x: 1 }
        }
    }
}

#[component]
fn DemoC(x: i32) -> Element {
    let result = Err("Error");

    result.throw()?;

    render! {
        h1 { "{x}" }
    }
}
