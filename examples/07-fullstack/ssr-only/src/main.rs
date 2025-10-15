//! This example showcases how to use Fullstack in a server-side rendering only context.
//!
//! This means we have no client-side bundle at all, and *everything* is rendered on the server.
//! You can still use signals, resources, etc, but they won't be reactive on the client.
//!
//! This is useful for static site generation, or if you want to use Dioxus Fullstack as a server-side
//! framework without the `rsx! {}` markup.
//!
//! To run this example, simply run `cargo run --package ssr-only` and navigate to `http://localhost:8080`.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| rsx! { Router::<Route> { } });
}

#[derive(Routable, Clone, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home,

    #[route("/post/:id")]
    Post { id: u32 },
}

#[component]
fn Home() -> Element {
    rsx! {
        h1 { "home"  }
        ul {
            li { a { href: "/post/1", "Post 1" } }
            li { a { href: "/post/2", "Post 2" } }
            li { a { href: "/post/3", "Post 3 (404)" } }
        }
    }
}

#[component]
fn Post(id: ReadSignal<u32>) -> Element {
    // You can return `HttpError` to return a specific HTTP status code and message.
    // `404 Not Found` will cause the server to return a 404 status code.
    //
    // `use_loader` will suspend the server-side rendering until the future resolves.
    let post_data = use_loader(move || get_post(id()))?;

    rsx! {
        h1 { "Post {id}" }
        p { "{post_data}" }
    }
}

#[get("/api/post/{id}")]
async fn get_post(id: u32) -> Result<String, HttpError> {
    match id {
        1 => Ok("first post".to_string()),
        2 => Ok("second post".to_string()),
        _ => HttpError::not_found("Post not found")?,
    }
}
