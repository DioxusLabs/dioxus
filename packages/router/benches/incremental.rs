#![allow(unused, non_snake_case)]

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dioxus_router::ssr::{DefaultRenderer, IncrementalRenderer};
use dioxus_ssr::Renderer;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("build 1000 routes", |b| {
        let mut renderer = IncrementalRenderer::builder(DefaultRenderer {
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
        .invalidate_after(Duration::from_secs(10))
        .build();

        b.iter(|| {
            for id in 0..100 {
                for id in 0..10 {
                    renderer
                        .render(Route::Post { id }, &mut std::io::sink())
                        .unwrap();
                }
            }
        })
    });
    c.bench_function("build 1000 routes no memory cache", |b| {
        let mut renderer = IncrementalRenderer::builder(DefaultRenderer {
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
        .memory_cache_limit(0)
        .invalidate_after(Duration::from_secs(10))
        .build();

        b.iter(|| {
            for id in 0..1000 {
                renderer
                    .render(Route::Post { id }, &mut std::io::sink())
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

                vdom.rebuild();

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
        b.iter(|| {
            let mut renderer = IncrementalRenderer::builder(DefaultRenderer {
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
            .build();

            renderer.pre_cache_static_routes::<Route>().unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

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
        for _ in 0..*id {
            div {
                "PostId: {id}"
            }
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

#[inline_props]
fn RenderPath(cx: Scope, path: Route) -> Element {
    let path = path.clone();
    render! {
        Router {
            config: || RouterConfig::default().history(MemoryHistory::with_initial_path(path))
        }
    }
}
