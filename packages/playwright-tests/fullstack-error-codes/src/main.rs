//! To render custom error pages, you can create a layout component that captures errors from routes
//! with an `ErrorBoundary` and display different content based on the error type.
//!
//! While capturing the error, we set the appropriate HTTP status code using `FullstackContext::commit_error_status`.
//! The router will then use this status code when doing server-side rendering (SSR).
//!
//! Any errors not captured by an error boundary will be handled by dioxus-ssr itself, which will render
//! a generic error page instead.

use dioxus::prelude::*;
use dioxus_fullstack::{FullstackContext, StatusCode};

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
            Link { to: Route::Blog { id: 2 }, "Go to blog post 2 (201)" }
            Link { to: Route::Blog { id: 3 }, "Go to blog post 3 (error)" }
            Link { to: Route::Blog { id: 4 }, "Go to blog post 4 (not found)" }
        }
    }
}

#[component]
fn Blog(id: u32) -> Element {
    match id {
        1 => rsx! { div { "Blog post 1" } },
        2 => {
            FullstackContext::commit_http_status(StatusCode::CREATED, None);
            rsx! { div { "Blog post 2" } }
        }
        3 => dioxus_core::bail!("An error occurred while loading the blog post!"),
        _ => HttpError::not_found("Blog post not found")?,
    }
}

/// In our `ErrorLayout` component, we wrap the `Outlet` in an `ErrorBoundary`. This lets us attempt
/// to downcast the error to an `HttpError` and set the appropriate status code.
///
/// The `commit_error_status` function will attempt to downcast the error to an `HttpError` and
/// set the status code accordingly. Note that you can commit any status code you want with `commit_http_status`.
///
/// The router will automatically set the HTTP status code when doing SSR.
#[component]
fn ErrorLayout() -> Element {
    rsx! {
        ErrorBoundary {
            handle_error: move |err: ErrorContext| {
                let http_error = FullstackContext::commit_error_status(err.error().unwrap());
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
