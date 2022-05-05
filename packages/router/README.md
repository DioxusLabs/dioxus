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
    <a href="https://dioxuslabs.com/router/guide">Guide (Release)</a>
    <span> | </span>
    <a href="https://dioxuslabs.com/nightly/router/guide">Guide (Master)</a>
  </h3>
</div>


Dioxus Router is a first-party Router for all your Dioxus Apps. It provides an
interface that works anywhere: across the browser, SSR, and natively.

```rust ,ignore
fn app() {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        fixed: vec![(
          String::from("blog"),
          Route {
              content: RcComponent(Blog),
              sub: Segment {
                  index: RcComponent(BlogList),
                  dynamic: DrParameter {
                      name: None,
                      key: "id",
                      content: RcComponent(BlogPost),
                      sub: None,
                  }
                  ..Default::default()
              }
              ..Default::default()
          }
        )]
        ..Default::default()
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            Outlet { },
        }
    })
}
```


## Resources

- See the [mdbook][guide]
- The [crates.io API][api]

[api]: https://docs.rs/dioxus-router
[guide]: https://dioxuslabs.com/router/guide
