use anyhow::Context;
use dioxus::prelude::*;
use dioxus_router::prelude::*;

/// When the user is redirected back to the app from the OIDC provider, the query string will contain an authorization code.
/// However, this code is not the access token. Instead, it must be exchanged for an access token. This means we need
/// to go back to the OIDC provider and make a request to exchange the code for a token.
///
/// So, on the login route, we parse the query string and exchange the code for a token.
#[component]
pub fn Login(cx: Scope, query_string: String) -> Element {
    let exchange = use_future(cx, query_string, |query_string| async move {
        let (_, auth_code) = form_urlencoded::parse(query_string.as_bytes())
            .find(|(key, _value)| key == "code")
            .context("No query string")?;

        crate::Auth::exchange_code(auth_code.to_string()).await
    });

    if crate::USER().logged_in() {
        return render! {
            div { "Sign in successful" }
            Link { to: crate::Route::Home, "Go back home" }
        };
    }

    if exchange.pending() {
        return render! { div { "Logging in..." } };
    }

    render! {
        div { "Error while attempting to log in" }
        Link {
            to: crate::Route::Home,
            onclick: move |_| crate::USER.write().logout(),
            "Go back home"
        }
    }
}
