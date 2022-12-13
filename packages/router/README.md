<div align="center">
  <h1>Dioxus Router</h1>
</div>


<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/v/dioxus.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/d/dioxus.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/jkelleyrtp/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>

  <!--Awesome -->
  <a href="https://github.com/dioxuslabs/awesome-dioxus">
    <img src="https://cdn.rawgit.com/sindresorhus/awesome/d7305f38d29fed78fa85652e3a63e154dd8e8829/media/badge.svg" alt="Awesome Page" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square" alt="Discord Link" />
  </a>
</div>



<div align="center">
  <h3>
    <a href="https://dioxuslabs.com"> Website </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/example-projects"> Examples </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/router"> Guide (Latest) </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/nightly/router"> Guide (Master) </a>
  </h3>
</div>

Dioxus Router is a first-party Router for all your Dioxus Apps. It provides a React-Router style interface that works anywhere: across the browser, SSR, and natively.

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


## Resources

- See the mdbook
- See the one-page brief
- See the guide on the doc site
- The crates.io API
