//! This example shows how to use the axum `Redirect` type to redirect the client to a different URL.
//!
//! On the web, a redirect will not be handled directly by JS, but instead the browser will automatically
//! follow the redirect. This is useful for redirecting to different pages after a form submission.
//!
//! Note that redirects returned to the client won't navigate the SPA to a new page automatically.
//! For managing a session or auth with client side routing, you'll need to handle that in the SPA itself.

use dioxus::{fullstack::Redirect, prelude::*};

fn main() {
    dioxus::launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Clone, PartialEq, Routable)]
enum Route {
    #[route("/")]
    Home,

    #[route("/blog")]
    Blog,
}

#[component]
fn Home() -> Element {
    rsx! {
        h1 { "Welcome home" }
        form {
            method: "post",
            action: "/api/old-blog",
            button { "Go to blog" }
        }
    }
}

#[component]
fn Blog() -> Element {
    rsx! {
        h1 { "Welcome to the blog!" }
    }
}

#[post("/api/old-blog")]
async fn redirect_to_blog() -> Result<Redirect> {
    Ok(Redirect::to("/blog"))
}
