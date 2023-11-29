use dioxus::prelude::*;

#[component]
pub fn Profile(cx: Scope) -> Element {
    let email = crate::USER()
        .email()
        .throw_with(cx, || anyhow::anyhow!("Not logged in"))?;

    render! {
        h1 { "Welcome to the admin page" }
        div { "Your email from the token is: {email}" }
    }
}
