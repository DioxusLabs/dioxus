# Redirects
When defining our routes we can easily tell the router to redirect to another
path:

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .fixed(
                "content",
                Route::new(RcComponent(Comp))
            )
            .fixed(
                "redirect",
                Route::new(RcRedirect(NtPath(String::from("/content"))))
            )
    });

    // ...
    # unimplemented!()
}
#
# fn Comp(cx: Scope) -> Element { unimplemented!() }
```
