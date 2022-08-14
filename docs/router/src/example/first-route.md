# Creating Our First Route
In this chapter, we will start utilizing Dioxus Router and add a homepage and a
404 page to our project.

## Fundamentals
Dioxus Router works based on a [`Router`] component, a route definition in
regular rust and [`Outlet`] components. If you've ever used [Vue Router],
you should feel at home with Dioxus Router.

First we need an actual page to route to! Let's add a homepage component:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
#
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome to the Dioxus Blog!" }
    })
}
```

## To Route or Not to Route
We want to use Dioxus Router to separate our application into different "pages".
Dioxus Router will then determine which page to render based on the URL path.

To start using Dioxus Router, we need to use the [`Router`] component. All hooks
and other components the Router provides can only be used as a descendant of
a [`Router`] component.

However, before we can add the [`Router`] we need to describe our routes in a
type it can understand:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# fn Home(cx: Scope) -> Element { unimplemented!() }

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        // we want our home page component to render as an index
        Segment::default().index(Home as Component)
    });

    cx.render(rsx! {
        p { "Hello, Dioxus!"}
    })
}
```

Now we can replace the `p { "Hello, Dioxus!" }` with our [`Router`]. We also
need to tell it where to render the content of the active route. Therefore we
nest an [`Outlet`] inside it.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# fn Home(cx: Scope) -> Element { unimplemented!() }

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default().index(Home as Component)
    });

    cx.render(rsx! {
        // new stuff starts here
        Router {
            routes: routes.clone() // pass in the routes we prepared before
            Outlet { }
        }
        // new stuff ends here
    })
}
```

If you head to your application's browser tab, you should now see the text
`Welcome to Dioxus Blog!` when on the root URL (`http://localhost:8080/`). If
you enter a different path for the URL, nothing should be displayed.

This is because we told Dioxus Router to render the `Home` component only when
the URL path is `/`. The _index_ functionality we used basically emulates how
web servers treat `index.html` files.

## What if a Route Doesn't Exist?
In our example Dioxus Router doesn't render anything. Many sites also have a
"404" page for when a URL path leads to nowhere. Dioxus Router can do this too!

First, we create a new `PageNotFound` component.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
#
fn PageNotFound(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
    })
}
```

Now to tell Dioxus Router to render our new component when no route exists.
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn PageNotFound(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(Home as Component)
            .fallback(PageNotFound as Component) // this is new
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            Outlet { }
        }
    })
}
```

Now when you go to a route that doesn't exist, you should see the page not found
text.

## Conclusion
In this chapter we learned how to create a route and tell Dioxus Router what
component to render when the URL path is `/`. We also created a 404 page to
handle when a route doesn't exist. Next, we'll create the blog portion of our
site. We will utilize nested routes and URL parameters.

[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[Vue Router]: https://router.vuejs.org/
