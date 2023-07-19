# Layouts

Layouts allow you to wrap all child routes in a component. This can be useful when creating something like a header that will be used in many different routes.

[`Outlet`] tells the router where to render content in layouts. In the following example,
the Index will be rendered within the [`Outlet`].

```rust, no_run
{{#include ../../examples/outlet.rs:outlet}}
```

The example above will output the following HTML (line breaks added for
readability):

```html
<header>header</header>
<h1>Index</h1>
<footer>footer</footer>
```
