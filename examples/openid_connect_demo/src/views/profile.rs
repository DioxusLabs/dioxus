use dioxus::prelude::*;

#[component]
pub fn Profile(cx: Scope) -> Element {
    let token = crate::USER()
        .profile()
        .throw_with(cx, || anyhow::anyhow!("Not logged in"))?;

    render! {
        h1 { "Welcome to the admin page" }
        div {
            "You have token"
            pre { "{token:?}"}
        }
    }
}
