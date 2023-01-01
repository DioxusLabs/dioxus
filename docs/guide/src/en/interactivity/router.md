# Router

In many of your apps, you'll want to have different "scenes". For a webpage, these scenes might be the different webpages with their own content. For a desktop app, these scenes might be different views in your app.

To unify these platforms, Dioxus provides a first-party solution for scene management called Dioxus Router.


## What is it?

For an app like the Dioxus landing page (https://dioxuslabs.com), we want to have several different scenes:

- Homepage
- Blog

Each of these scenes is independent – we don't want to render both the homepage and blog at the same time.

The Dioxus router makes it easy to create these scenes. To make sure we're using the router, add the `dioxus-router` package to your `Cargo.toml`.

```shell
cargo add dioxus-router
```


## Using the router

Unlike other routers in the Rust ecosystem, our router is built declaratively. This makes it possible to compose our app layout simply by arranging components.

```rust
rsx!{
    // All of our routes will be rendered inside this Router component
    Router {
        // if the current location is "/home", render the Home component
        Route { to: "/home", Home {} }
        // if the current location is "/blog", render the Blog component
        Route { to: "/blog", Blog {} }
    }
}
```

Whenever we visit this app, we will get either the Home component or the Blog component rendered depending on which route we enter at. If neither of these routes match the current location, then nothing will render.

We can fix this one of two ways:

- A fallback 404 page

```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
        //  if the current location doesn't match any of the above routes, render the NotFound component
        Route { to: "", NotFound {} }
    }
}
```


- Redirect 404 to home

```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
        //  if the current location doesn't match any of the above routes, redirect to "/home"
        Redirect { from: "", to: "/home" }
    }
}
```

## Links

For our app to navigate these routes, we can provide clickable elements called Links. These simply wrap `<a>` elements that, when clicked, navigate the app to the given location.


```rust
rsx!{
    Link {
        to: "/home",
        "Go home!"
    }
}
```

## More reading

This page is just a very brief overview of the router. For more information, check out [the router book](https://dioxuslabs.com/router/guide/) or some of [the router examples](https://github.com/DioxusLabs/dioxus/blob/master/examples/router.rs).
