# Links & Navigation

When we split our app into pages, we need to provide our users with a way to
navigate between them. On regular web pages, we'd use an anchor element for that,
like this:

```html
<a href="/other">Link to an other page</a>
```

However, we cannot do that when using the router for three reasons:

1. Anchor tags make the browser load a new page from the server. This takes a
   lot of time, and it is much faster to let the router handle the navigation
   client-side.
2. Navigation using anchor tags only works when the app is running inside a
   browser. This means we cannot use them inside apps using Dioxus Desktop.
3. Anchor tags cannot check if the target page exists. This means we cannot
   prevent accidentally linking to non-existent pages.

To solve these problems, the router provides us with a [`Link`] component we can
use like this:

```rust, no_run
{{#include ../../../examples/links.rs:nav}}
```

The `target` in the example above is similar to the `href` of a regular anchor
element. However, it tells the router more about what kind of navigation it
should perform. It accepts something that can be converted into a
[`NavigationTarget`]:

- The example uses a Internal route. This is the most common type of navigation.
  It tells the router to navigate to a page within our app by passing a variant of a [`Routable`] enum. This type of navigation can never fail if the link component is used inside a router component.
- [`External`] allows us to navigate to URLs outside of our app. This is useful
  for links to external websites. NavigationTarget::External accepts an URL to navigate to. This type of navigation can fail if the URL is invalid.

> The [`Link`] accepts several props that modify its behavior. See the API docs
> for more details.
