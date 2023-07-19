# History Buttons

Some platforms, like web browsers, provide users with an easy way to navigate
through an app's history. They have UI elements or integrate with the OS.

However, native platforms usually don't provide such amenities, which means that
apps wanting users to have access to them, need to implement them. For this
reason, the router comes with two components, which emulate a browser's back and
forward buttons:

- [`GoBackButton`](https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.GoBackButton.html)
- [`GoForwardButton`](https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.GoForwardButton.html)

> If you want to navigate through the history programmatically, take a look at
> [`programmatic navigation`](./navigation/programmatic.md).

```rust, no_run
{{#include ../../examples/history_buttons.rs:history_buttons}}
```

As you might know, browsers usually disable the back and forward buttons if
there is no history to navigate to. The router's history buttons try to do that
too, but depending on the [history provider] that might not be possible.

Importantly, neither [`WebHistory`] supports that feature.
This is due to limitations of the browser History API.

However, in both cases, the router will just ignore button presses, if there is
no history to navigate to.

Also, when using [`WebHistory`], the history buttons might
navigate a user to a history entry outside your app.
