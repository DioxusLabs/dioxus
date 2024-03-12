use std::time::Duration;

use dioxus::prelude::*;

use dioxus_ssr::incremental::{DefaultRenderer, IncrementalRendererConfig};

#[tokio::main]
async fn main() {
    let mut renderer = IncrementalRendererConfig::new()
        .static_dir("./static")
        .invalidate_after(Duration::from_secs(10))
        .build();

    println!(
        "SITE MAP:\n{}",
        Route::SITE_MAP
            .iter()
            .flat_map(|route| route.flatten().into_iter())
            .map(|route| {
                route
                    .iter()
                    .map(|segment| segment.to_string())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .collect::<Vec<_>>()
            .join("\n")
    );

    // This function is available if you enable the ssr feature
    // on the dioxus::router crate.
    pre_cache_static_routes::<Route, _>(
        &mut renderer,
        &DefaultRenderer {
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
        },
    )
    .await
    .unwrap();
}

#[component]
fn Blog() -> Element {
    rsx! {
        div {
            "Blog"
        }
    }
}

#[component]
fn Post(id: usize) -> Element {
    rsx! {
        div {
            "PostId: {id}"
        }
    }
}

#[component]
fn PostHome() -> Element {
    rsx! {
        div {
            "Post"
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
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
