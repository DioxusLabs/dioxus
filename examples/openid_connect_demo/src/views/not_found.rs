use dioxus::prelude::*;

#[component]
pub fn NotFound(cx: Scope, route: Vec<String>) -> Element {
    render! {
        div{
            {route.join("")}
        }
    }
}
