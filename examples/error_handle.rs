use dioxus::{dioxus_core::CapturedError, prelude::*};

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    render! {
        ErrorBoundary {
            handle_error: |error: CapturedError| render! {"Found error {error}"},
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
