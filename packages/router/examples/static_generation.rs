#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use dioxus_router::ssr::{DefaultRenderer, IncrementalRendererConfig};

fn main() {
    let mut renderer = IncrementalRendererConfig::new(DefaultRenderer {
        before_body: r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width,
            initial-scale=1.0">
            <title>Dioxus Application</title>
        </head>
        <body>"#
            .to_string(),
        after_body: r#"</body>
        </html>"#
            .to_string(),
    })
    .static_dir("./static")
    .memory_cache_limit(5)
    .build();

    renderer.pre_cache_static::<Route>();

    for _ in 0..2 {
        for id in 0..10 {
            renderer.render(Route::Post { id });
        }
    }
}

#[inline_props]
fn Blog(cx: Scope) -> Element {
    render! {
        div {
            "Blog"
        }
    }
}

#[inline_props]
fn Post(cx: Scope, id: usize) -> Element {
    render! {
        div {
            "PostId: {id}"
        }
    }
}

#[inline_props]
fn PostHome(cx: Scope) -> Element {
    render! {
        div {
            "Post"
        }
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    render! {
        div {
            "Home"
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[nest("/blog")]
        #[route("/")]
        Blog {},
        #[route("/post/index")]
        PostHome {},
        #[route("/post/:id")]
        Post {
            id: usize,
        },
    #[end_nest]
    #[route("/")]
    Home {},
}
