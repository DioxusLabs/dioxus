use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let a = 123;
    let b = 456;
    let c = 789;

    render! {
        Component { a, b, c }
        Component { a, ..ComponentProps { a: 1, b: 2, c: 3 } }
    }
}

#[component]
fn Component(cx: Scope, a: i32, b: i32, c: i32) -> Element {
    render! {
        div { "{a}" }
        div { "{b}" }
        div { "{c}" }
    }
}
