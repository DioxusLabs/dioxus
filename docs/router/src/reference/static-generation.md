# Static Generation

## Getting the Sitemap

The [`Routable`] trait includes an associated [`SITE_MAP`] constant that contains the map of all of the routes in the enum.

By default, the sitemap is a tree of (static or dynamic) RouteTypes, but it can be flattened into a list of individual routes with the `.flatten()` method.

## Generating a Sitemap

To statically render pages, we need to flatten the route tree and generate a file for each route that contains only static segments:

```rust, no_run
{{#include ../../../../packages/router/examples/static_generation.rs}}
```
