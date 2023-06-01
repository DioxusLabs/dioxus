# Redirects

In some cases, we may want to redirect our users to another page whenever they
open a specific path. We can tell the router to do this with the `#[redirect]`
attribute.

The `#[redirect]` attribute accepts a route and a closure with all of the parameters defined in the route. The closure must return a [`NavigationTarget`].

In the following example, we will redirect everybody from `/myblog` and `/myblog/:id` to `/blog` and `/blog/:id` respectively

```rust, no_run
{{#include ../../examples/full_example.rs:router}}
```
