<div align="center">
  <h1>Dioxus Router</h1>
</div>


<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus-router">
    <img src="https://img.shields.io/crates/v/dioxus-router.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus-router">
    <img src="https://img.shields.io/crates/d/dioxus-router.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus-router">
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
    <a href="https://dioxuslabs.com">Website</a>
    <span> | </span>
    <a href="https://dioxuslabs.com/router">Guide (Release)</a>
    <span> | </span>
    <a href="https://dioxuslabs.com/nightly/router"> Guide (Master) </a>
  </h3>
</div>


Dioxus Router is a first-party Router for all your Dioxus Apps. It provides an
interface that works anywhere: across the browser, SSR, and natively.

```rust
use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn App(cx: Scope) -> Element {
    // declare the routes of the app
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Index as Component) // when the path is '/'
            .fixed("other", Route::new(Other as Component)) // when the path is `/other`
    });

    cx.render(rsx! {
        // render the router and give it the routes
        Router {
            routes: routes.clone(),

            // give the router a place to render the content
            Outlet { }
        }
    })
}

fn Index(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Example" }
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Some content" }
    })
}
```
