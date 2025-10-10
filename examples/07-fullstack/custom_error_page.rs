//! We can use the `#[error]` attribute on the Dioxus Router to customize the error page.
//!
//! The `#[error]` attribute catches any errors while rendering a route and returns a `RoutingError`.
//! It's up to you to decide how to handle the error and what to show the user.
//!
//! The Router will automatically downcast any errors into something that returns a `StatusCode`,
//! and if it can't it, it will return a 500 Internal Server Error. When used in SSR, the Router will
//! take these status codes and call `dioxus::fullstack::set_status` to set the HTTP status code appropriately.
//!
//! Note that the `#[error]` attribute applies *per layout*. A layout can have a dedicated error handler
//! that prevents the error from bubbling up to the parent layout.
//!
//! An `#[error]` handler can also return an `Error` which then propagates up to the parent layout.
//! This lets certain errors like `NotAuthorized` be handled locally, while more serious errors
//! like `InternalServerError` can be handled by the root layout.

use dioxus::prelude::*;
use dioxus_fullstack::StatusCode;

fn main() {
    dioxus::launch(|| {
        rsx! {
            Router::<Route> {}
        }
    });
}

#[derive(Routable, PartialEq, Clone, Debug)]
enum Route {
    #[layout(ErrorLayout)]
    #[route("/")]
    Home,

    #[route("/blog/:id")]
    Blog { id: u32 },
}

#[component]
fn Home() -> Element {
    rsx! {
        div { "Welcome to the home page!" }
        div { display: "flex", flex_direction: "column",
            Link { to: Route::Blog { id: 1 }, "Go to blog post 1" }
            Link { to: Route::Blog { id: 2 }, "Go to blog post 2" }
            Link { to: Route::Blog { id: 3 }, "Go to blog post 3 (error)" }
            Link { to: Route::Blog { id: 4 }, "Go to blog post 4 (not found)" }
        }
    }
}

#[component]
fn Blog(id: u32) -> Element {
    match id {
        1 => rsx! { div { "Blog post 1" } },
        2 => rsx! { div { "Blog post 2" } },
        3 => dioxus_core::bail!("An error occurred while loading the blog post!"),
        _ => HttpError::not_found("Blog post not found")?,
    }
}

/// In our `ErrorPage` component, we can match on the status code and display
/// different content based on the error type.
///
/// The router will automatically set the HTTP status code when doing SSR.
#[component]
fn ErrorLayout() -> Element {
    rsx! {
        ErrorBoundary {
            handle_error: move |err: ErrorContext| {
                let http_error = dioxus_fullstack::FullstackContext::commit_error_status(err.error().unwrap());
                match http_error.status {
                    StatusCode::NOT_FOUND => rsx! { div { "404 - Page not found" } },
                    StatusCode::UNAUTHORIZED => rsx! { div { "401 - Unauthorized" } },
                    StatusCode::INTERNAL_SERVER_ERROR => rsx! { div { "500 - Internal Server Error" } },
                    _ => rsx! { div { "An unknown error occurred" } },
                }
            },
            Outlet::<Route> {}
        }
    }
}
