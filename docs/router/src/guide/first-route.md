# Creating Our First Route
In this chapter, we will continue off of our new Dioxus project to create a homepage and start utilizing Dioxus Router!

### Fundamentals
Dioxus Router works based on a router and route component. If you've ever used [React Router](https://reactrouter.com/), you should feel at home with Dioxus Router.

To get started, import the ``Router`` and ``Route`` components.
```rs
use dioxus::{
    prelude::*,
    router::{Route, Router}
}
```
We also need an actual page to route to! Add a homepage component:
```rs
fn homepage(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Welcome to Dioxus Blog!" }
    })
}
```

### To Route or Not to Route
We want to use Dioxus Router to seperate our application into different "pages". Dioxus Router will then determine which page to render based on the URL path.

To start using Dioxus Router, we need to use the ``Router`` component.
Replace the ``p { "Hello, wasm!" }`` in your ``app`` component with a Router component:
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {} // NEW
    })
}
```
Now we have established a router and we can create our first route. We will be creating a route for our homepage component we created earlier.
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            Route { to: "/", self::homepage {}} // NEW
        }
    })
}
```
If you head to your application's browser tab, you should see the text ``Welcome to Dioxus Blog!`` when on the root URL (``http://localhost:8080/``). If you enter a different path for the URL, nothing should be displayed.

This is because we told Dioxus Router to render the ``homepage`` component only when the URL path is ``/``. You can tell Dioxus Router to render any kind of component such as a ``div {}``.

### What if a Route Doesn't Exist?
In our example Dioxus Router doesn't render anything. If we wanted to, we could tell Dioxus Router to render a component all the time! Try it out:
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" } // NEW
            Route { to: "/", self::homepage {}}
        }
    })
}
```
We will go into more detail about this in the next chapter.

Many sites also have a "404" page for when a URL path leads to nowhere. Dioxus Router can do this too! Create a new ``page_not_found`` component.
```rs
fn page_not_found(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Oops! The page you are looking for doesn't exist!" }
    })
}
```

Now to tell Dioxus Router to render our new component when no route exists. Create a new route with a path of nothing:
```rs
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            p { "-- Dioxus Blog --" }
            Route { to: "/", self::homepage {}}
            Route { to: "", self::page_not_found {}} // NEW
        }
    })
}
```
Now when you go to a route that doesn't exist, you should see the page not found text and the text we told Dioxus Router to render all the time.
```
// localhost:8080/abc

-- Dioxus Blog --
Oops! The page you are looking for doesn't exist!
```

> Make sure you put your empty route at the bottom or else it'll override any routes below it!

### Conclusion
In this chapter we learned how to create a route and tell Dioxus Router what component to render when the URL path is equal to what we specified. We also created a 404 page to handle when a route doesn't exist. Next, we'll create the blog portion of our site. We will utilize nested routes and URL parameters.