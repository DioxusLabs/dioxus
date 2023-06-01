# Layouts

Layouts allow you to 

[`Outlet`]s tell the router where to render content in layouts. In the following example,
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
