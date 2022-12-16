# Redirection Limit Failure

The router enforces a limit of 25 redirects during a single routing navigation.
This is done to prevent infinite loops. If your app breaches that limit, you
should reorganize its routes to reduce the number of redirects.

> Users cannot trigger a redirection. If the redirection limit is breached, your
> app (or the router) has a bug.

> The [`on_update`](../routing-update-callback.md) callback doesn't count
> towards the limit, and resets it. You may have 25 redirects, then add an other
> one via the callback, and then have another 25.

The router reacts to a breach differently, depending on our apps build kind.

## Debug
When running a debug build, the router will `panic` whenever the redirecion
limit is breached. This ensures that we notice these problems when we are
testing our application.

## Release
When running a release build, the router can't just `panic`, as that would be a
horrible user experience. Instead, it changes to show some fallback content.

> You can detect if the router is in the redirection limit failure handling
> state by [checking](../navigation/name.md#check-if-a-name-is-present) if the
> [`FailureRedirectionLimit`] name is present.

The default fallback explains to the user that an error occurred and asks them
to report the bug to the app developer.

You can override it by setting the `failure_redirection_limit` value of the
[`RouterConfiguration`].

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
fn RedirectionLimitFallback(cx: Scope) -> Element {
    render! {
        h1 { "Redirection limit breached!" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            failure_redirection_limit: comp(RedirectionLimitFallback),
            ..Default::default()
        },
        &|| Segment::empty()
    );

    render! {
        Outlet { }
    }
}
```

[`FailureRedirectionLimit`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/prelude/struct.FailureRedirectionLimit.html
[`RouterConfiguration`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/struct.RouterConfiguration.html
