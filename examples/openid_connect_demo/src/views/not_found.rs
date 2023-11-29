use dioxus::prelude::*;

#[component]
pub fn NotFound(cx: Scope, route: Vec<String>) -> Element {
    render! {
        h1 { "Err 404: Route not found" }
        div { route.join("") }
    }
}
