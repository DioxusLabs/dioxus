use crate::{Route, USER};
use dioxus::prelude::*;
use dioxus_router::prelude::{Link, Outlet};

/// This layout wraps all the routes with a header.
///
/// You've likely seen this with other apps, where the header is used to show the user's name and a logout button.
/// In this case, we use it to show a login button if the user is not logged in, and a logout button if they are.
///
/// In lieu of a name, we show the token directly.
#[component]
pub fn AuthHeader(cx: Scope) -> Element {
    // If the user is logged out, load the OIDC client and then show a Login button.
    // We'll make the button open the OIDC provider's login page.
    //
    // We might want to consider a design where the page is suspended until the client is loaded
    _ = use_future(cx, (), |_| async move { crate::Auth::load_client().await });

    render! {
        div {
            match USER().logout_url() {
                Some(uri) => render! {
                    Link {
                        to: "{uri}",
                        onclick: move |_| USER.write().logout(),
                        "Log out"
                    }
                },
                None => render! {
                    Link {
                        // While the client is loading, we'll still show the login button, but it won't do anything.
                        to: USER().login_url().unwrap_or_default(),
                        onclick: move |_| USER.write().login(),
                        "Log in"
                    }
                },
            }
            Outlet::<Route> {}
        }
    }
}
