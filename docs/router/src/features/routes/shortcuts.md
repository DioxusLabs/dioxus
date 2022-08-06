# Shortcuts

As you might have noticed, defining routes can be quite verbose. For example,
this is how we'd declare a redirect to an external page:

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn App(cx: Scope) -> Element {
let routes = use_segment(&cx, || {
    Segment::new()
        .fixed(
            "redirect",
            Route::new(
                RcRedirect(ExternalTarget(String::from("https://dioxuslabs.com")))
            )
        )
});
# unimplemented!();
# }
```

As we can see, this simple task gets quite long, because we have to:
- specify we want a [`Route`],
- create a [`NavigationTarget::ExternalTarget`],
- and manually have to convert our value to a `String`.

However, the router allows us to simplify this declaration quite a bit:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn App(cx: Scope) -> Element {
let routes = use_segment(&cx, || {
    Segment::new().fixed("redirect", "https://dioxuslabs.com")
});
# unimplemented!();
# }
```

As the example shows the router can infer quite a bit of information from the
fact that we pass it a `str`. Based on whether the target url starts with
`http://` or `https://`, it can even tell if the target is external.

Similar shortcuts exist for many of the route definition types.
