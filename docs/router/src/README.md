# Introduction
Whether or not you are building a website, desktop app, or mobile app,
organizing your app's views into "pages" can be an effective method for
organization and maintainability.

For this purpose, Dioxus provides a built-in router. To start utilizing it,
enable the `router` feature in your `Cargo.toml` file:
```toml
[dependencies]
dioxus = { .., features = [.., "router"] }
```

> If you are not familiar with Dioxus itself, check out the [Dioxus book][db]
> first.

This book is intended to get you up to speed with Dioxus Router. It is split
into three sections:
1. The _[Basics](./basics/adding-the-router.md)_ coverer everything you need to
   use the router.
2. In _Advanced_ you will learn about more in-depth concepts, that are not
   necessary for all use cases.
3. If you prefer a learning-by-doing approach, you can check ouf the
   _[example project](./example/introduction.md)_. It guides you through
   creating a dioxus app and setting up a router, and covers some basic concepts
   as well.

> Please note that this is not the only documentation for the Dioxus Router. You
> can also check out the [API Docs][api].

[api]: https://docs.rs/dioxus-router/
[Basics]: /basics/adding-the-router.md
[db]: https://dioxuslabs.com/guide/
[example project]: /example/index.md
