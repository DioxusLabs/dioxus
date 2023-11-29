pub(crate) mod constants;
pub(crate) mod model;
pub(crate) mod state;
pub(crate) mod views;

// Re-export Auth and USER to the rest of the app
pub use state::{Auth, USER};

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use views::{header::AuthHeader, home::Home, login::Login, not_found::NotFound, profile::Profile};

/// The app's router, using the modern Dioxus Router API.
///
/// We show how to use the layout attribute to wrap the routes with a header that provides a typical login/logout flow.
/// Within the routes themselves, we also show how to tap into the state of the router to show a different view if the
/// user is logged in or not.
///
/// We also demonstrate that Layouts can also be used to guard routes from being accessed if the user is not logged in.
/// In practice, this is done by "throwing" an error from a child component.
///
/// To handle OIDC requests, we use the login route to parse the query string and exchange the auth code for a token.
#[rustfmt::skip]
#[derive(Routable, Clone)]
pub enum Route {
    #[layout(AuthHeader)]
        // Show a different view based on the login state of the app
        #[route("/")]
        Home,

        // Throw an error if the user is not logged in to the parent layout
        #[route("/profile")]
        Profile,

        // Handle login redirects from the oidc service
        #[route("/login?:query_string")]
        Login { query_string: String },
    #[end_layout]

    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

fn main() {
    console_error_panic_hook::set_once();
    dioxus_logger::init(log::LevelFilter::Info).unwrap();
    dioxus_web::launch(|cx| render! { Router::<Route> {} });
}
