use dioxus::prelude::*;

#[component]
pub fn NotFound(cx: Scope, route: Vec<String>) -> Element {
    let routes = route.join("");
    render! {rsx! {div{routes}}}
}
