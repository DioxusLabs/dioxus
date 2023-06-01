# Routing Update Callback

In some cases, we might want to run custom code when the current route changes.
For this reason, the [`RouterConfig`] exposes an `on_update` field.

## How does the callback behave?

The `on_update` is called whenever the current routing information changes. It
is called after the router updated its internal state, but before dependent components and hooks are updated.

If the callback returns a [`NavigationTarget`], the router will replace the
current location with the specified target. It will not call the
`on_update` again.

If at any point the router encounters a
[navigation failure](./failures/index.md), it will go to the appropriate state
without calling the `on_update`. It doesn't matter if the invalid target
initiated the navigation, was found as a redirect target, or was returned by the
`on_update` itself.

## Code Example

```rust, no_run
{{#include ../../examples/routing_update.rs:router}}
```
