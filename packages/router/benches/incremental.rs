#![allow(unused)]

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use dioxus_ssr::Renderer;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("build 1000 routes", |b| {
        let mut renderer = IncrementalRenderer::builder()
            .static_dir("./static")
            .invalidate_after(Duration::from_secs(10))
            .build();
        let wrapper = DefaultRenderer {
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
        };

        b.iter(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                for id in 0..1000 {
                    render_route(
                        &mut renderer,
                        Route::Post { id },
                        &mut tokio::io::sink(),
                        |_| Box::pin(async move {}),
                        &wrapper,
                    )
                    .await
                    .unwrap();
                }
            })
        })
    });

    c.bench_function("build 1000 routes no memory cache", |b| {
        let wrapper = DefaultRenderer {
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
        };
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut renderer = IncrementalRenderer::builder()
                    .static_dir("./static")
                    .memory_cache_limit(0)
                    .invalidate_after(Duration::from_secs(10))
                    .build();

                for id in 0..1000 {
                    render_route(
                        &mut renderer,
                        Route::Post { id },
                        &mut tokio::io::sink(),
                        |_| Box::pin(async move {}),
                        &wrapper,
                    )
                    .await
                    .unwrap();
                }
            })
    });
    c.bench_function("build 1000 routes no cache", |b| {
        let mut renderer = Renderer::default();

        b.iter(|| {
            for id in 0..1000 {
                let mut vdom = VirtualDom::new_with_props(
                    RenderPath,
                    RenderPathProps::builder().path(Route::Post { id }).build(),
                );

                vdom.rebuild_in_place();

                struct Ignore;

                impl std::fmt::Write for Ignore {
                    fn write_str(&mut self, s: &str) -> std::fmt::Result {
                        Ok(())
                    }
                }

                renderer.render_to(&mut Ignore, &vdom).unwrap();
            }
        })
    });
    c.bench_function("cache static", |b| {
        let wrapper = DefaultRenderer {
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
        };
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut renderer = IncrementalRenderer::builder()
                    .static_dir("./static")
                    .build();

                pre_cache_static_routes::<Route, _>(&mut renderer, &wrapper)
                    .await
                    .unwrap();
            })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

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
        for _ in 0..id {
            div {
                "PostId: {id}"
            }
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

#[component]
fn RenderPath(path: Route) -> Element {
    let path = path.clone();
    rsx! {
        Router::<Route> {
            config: || RouterConfig::default().history(MemoryHistory::with_initial_path(path))
        }
    }
}
