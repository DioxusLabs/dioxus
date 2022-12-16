# Creating Our First Route
In this chapter, we will start utilizing Dioxus Router and add a homepage and a
404 page to our project.

## Fundamentals
Dioxus Router works based on a [`use_router`] hook, a route definition in pure
rust and [`Outlet`] components. If you've ever used [Vue Router], you should
feel right at home with Dioxus Router.

First we need an actual page to route to! Let's add a homepage component:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
#
fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Welcome to the Dioxus Blog!" }
    }
}
```

## To Route or Not to Route
We want to use Dioxus Router to separate our application into different "pages".
Dioxus Router will then determine which page to render based on the URL path.

To start using Dioxus Router, we need to use the [`use_router`] hook. All other
hooks and components the router provides can only be used as a descendant of a
component calling [`use_router`].

The [`use_router`] hook takes three arguments:
1. `cx`, which is a common argument for all hooks.
2. A [`RouterConfiguration`], which allows us to modify its behavior.
3. A definition of all routes the application contains, in the form of its root
   [`Segment`].

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# fn Home(cx: Scope) -> Element { unimplemented!() }

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration::default(),
        &|| Segment::content(comp(Home))
    );

    render! {
        Outlet { }
    }
}
```

If you head to your application's browser tab, you should now see the text
`Welcome to Dioxus Blog!` when on the root URL (`http://localhost:8080/`). If
you enter a different path for the URL, nothing should be displayed.

This is because we told Dioxus Router to render the `Home` component only when
the URL path is `/`. The _index_ (`Segment::content()`) functionality we used
basically emulates how web servers treat `index.html` files.

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
    render! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
    }
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
    use_router(
        cx,
        &|| RouterConfiguration::default(),
        &|| {
            Segment::content(comp(Home))
                .fallback(comp(PageNotFound)) // this is new
        }
    );

    render! {
        Outlet { }
    }
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
[`RouterConfiguration`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/struct.RouterConfiguration.html
[`Segment`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/struct.Segment.html
[`use_router`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_router.html
[Vue Router]: https://router.vuejs.org/
