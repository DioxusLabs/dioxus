# Creating Our First Route
In this chapter, we will continue off of our new Dioxus project to create a
homepage and start utilizing Dioxus Router!

### Fundamentals
Dioxus Router works based on a router and route component. If you've ever used
[Vue Router](https://router.vue.com/), you should feel at home with Dioxus
Router.

In the previous chapter we imported the dioxus prelude. When the `router`
feature is active, this also imports the components and Types we need for the
router

We also need an actual page to route to! Add a homepage component:
```rust,ignore
#[allow(non_snake_case)]
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome to the Dioxus Blog!" }
    })
}
```

### To Route or Not to Route
We want to use Dioxus Router to seperate our application into different "pages".
Dioxus Router will then determine which page to render based on the URL path.

To start using Dioxus Router, we need to use the `Router` component. All hooks
and other components the Router provides can only be used as a descendant of
a `Router` component.

Before we can add the `Router` we need to describe our routes in a type it can
understand:
```rust,ignore
fn app(cx: Scope) -> Element {
    // this is new
    let routes = cx.use_hook(|_| Segment {
        // we want our home page component to render as an index
        index: RcComponent(Home),
        // we don't care about any other field
        ..Default::default()
    });

    cx.render(rsx! {
        p { "Hello, wasm!"}
    })
}
```

Now we can replace the `p { "Hello, wasm!" }` with our router:
```rust,ignore
fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        ..Default::default()
    });

    cx.render(rsx! {
        Router { // this is new
            routes: routes // pass in the routes we prepared before
        }
    })
}
```

At last, we need to tell the router where to render the component for the active
route:
```rust,ignore
fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        ..Default::default()
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            Outlet { } // this is new
        }
    })
}
```

If you head to your application's browser tab, you should see the text
`Welcome to Dioxus Blog!` when on the root URL (`http://localhost:8080/`). If
you enter a different path for the URL, nothing should be displayed.

This is because we told Dioxus Router to render the `Home` component only when
the URL path is `/`.

### What if a Route Doesn't Exist?
In our example Dioxus Router doesn't render anything. Many sites also have a
"404" page for when a URL path leads to nowhere. Dioxus Router can do this too!
Create a new `PageNotFound` component.
```rust,ignore
#[allow(non_snake_case)]
fn PageNotFound(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
    })
}
```

Now to tell Dioxus Router to render our new component when no route exists.
```rust,ignore
fn app(cx: Scope) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        ..Default::default()
    });

    cx.render(rsx! {
        Router {
            routes: routes,
            fallback: RcComponent(PageNotFound), // this is new
            Outlet { }
        }
    })
}
```

Now when you go to a route that doesn't exist, you should see the page not found
text.

### Conclusion
In this chapter we learned how to create a route and tell Dioxus Router what
component to render when the URL path is `/`. We also created a 404 page to
handle when a route doesn't exist. Next, we'll create the blog portion of our
site. We will utilize nested routes and URL parameters.
