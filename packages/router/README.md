# Dioxus Router

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-router.svg
[crates-url]: https://crates.io/crates/dioxus-router

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE

[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster

[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/docs/0.3/guide/en/) |
[API Docs](https://docs.rs/dioxus-router/latest/dioxus_router) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

Dioxus Router is a first-party Router for all your Dioxus Apps. It provides an
interface similar to React Router, but takes advantage of types for more
expressiveness.

```rust ,no_run
use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| Default::default(),
        &|| Segment::content(comp(Index)).fixed(
            "blog",
            Route::content(comp(Blog)).nested(
                Segment::content(comp(BlogList))
                    .catch_all((comp(BlogPost), BlogPost))
            )
        )
    );

    render! {
        Outlet { }
    }
}

fn Index(cx: Scope) -> Element {
    render! {
        h1 { "Index" }
        Link {
            target: "/blog",
            "Go to the blog"
        }
    }
}

fn Blog(cx: Scope) -> Element {
    render! {
        h1 { "Blog" }
        Outlet { }
    }
}

fn BlogList(cx: Scope) -> Element {
    render! {
        h2 { "List of blog posts" }
        Link {
            target: "/blog/1",
            "Blog post 1"
        }
        Link {
            target: "/blog/1",
            "Blog post 2"
        }
    }
}

fn BlogPost(cx: Scope) -> Element {
    render! {
        h2 { "Blog Post" }
    }
}
```

You need to enable the right features for the platform you're targeting since these are not determined automatically!

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
