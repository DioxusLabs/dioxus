# Introduction
Whether or not you are building a website, desktop app, or mobile app,
splitting your app's views into "pages" can be an effective method for
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
into two sections:
1. The [Topics](./topics/index.md) part explains individual topics in depth. You
   can read them start to finish, or you can read individual chapters in
   whatever order you want.
2. If you prefer a learning-by-doing approach, you can check ouf the
   _[example project](./example/introduction.md)_. It guides you through
   creating a dioxus app, setting up the router and using some of its
   functionality.

> Please note that this is not the only documentation for the Dioxus Router. You
> can also check out the [API Docs][api].

[api]: https://docs.rs/dioxus-router/
[db]: https://dioxuslabs.com/guide/
