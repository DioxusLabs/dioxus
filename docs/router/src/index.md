# Introduction
Whether or not you are building a website, desktop app, or mobile app,
splitting your app's views into "pages" can be an effective method for
organization and maintainability.

For this purpose, Dioxus provides a router. To start utilizing it, add it as a
dependency in your `Cargo.toml` file:
```toml
[dependencies]
# use this for native apps
dioxus-router = "*"
#use this for web apps
dioxus-router = { version = "*", features = ["web"] }

# in both cases replace * with the current version
```

You can also use the `cargo` command to add the dependency:
```sh
$ cargo add dioxus-router
$ cargo add dioxus-router --features web
```

> If you are not familiar with Dioxus itself, check out the [Dioxus book][db]
> first.

This book is intended to get you up to speed with Dioxus Router. It is split
into two sections:
1. The [Features](./features/index.md) part explains individual features in
   depth. You can read it start to finish, or you can read individual chapters
   in whatever order you want.
2. If you prefer a learning-by-doing approach, you can check ouf the
   _[example project](./example/introduction.md)_. It guides you through
   creating a dioxus app, setting up the router and using some of its
   functionality.

> Please note that this is not the only documentation for the Dioxus Router. You
> can also check out the [API Docs][api].

[api]: https://docs.rs/dioxus-router/
[db]: https://dioxuslabs.com/guide/
