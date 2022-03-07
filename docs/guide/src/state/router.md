# Router

In many of your apps, you'll want to have different "scenes". For a webpage, these scenes might be the different webpages with their own content.

You could write your own scene management solution - quite simply too. However, to save you the effort, Dioxus supports a first-party solution for scene management called Dioxus Router.


## What is it?

For an app like the Dioxus landing page (https://dioxuslabs.com), we want to have different pages. A quick sketch of an app would be something like:

- Homepage
- Blog
- Example showcase

Each of these scenes is independent - we don't want to render both the homepage and blog at the same time.

This is where the router crates come in handy. To make sure we're using the router, simply add the `"router"` feature to your dioxus dependency.

```toml
[dependencies]
dioxus = { version = "0.2", features = ["desktop", "router"] }
```


## Using the router

Unlike other routers in the Rust ecosystem, our router is built declaratively. This makes it possible to compose our app layout simply by arranging components.

```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
    }
}
```

Whenever we visit this app, we will get either the Home component or the Blog component rendered depending on which route we enter at. If neither of these routes match the current location, then nothing will render.

We can fix this one of two ways:

- A fallback 404 page
-
```rust
rsx!{
    Router {
        Route { to: "/home", Home {} }
        Route { to: "/blog", Blog {} }
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

This page is just meant to be a very brief overview of the router to show you that there's a powerful solution already built for a very common problem. For more information about the router, definitely check out its book or check out some of the examples.

The router has its own documentation! [Available here](https://dioxuslabs.com/router_guide/).
