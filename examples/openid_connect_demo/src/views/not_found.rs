use dioxus::prelude::*;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    render! {
        div{
            {route.join("")}
        }
    }
}
