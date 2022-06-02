# Named Navigation

When creating large applications, it can become difficult to keep track of all
routes and how to navigate to them. It also can be hard to find all links to
them, which makes it difficult to change paths.

To solve these problems, the router implements named navigation. When we define
our routes we can give them arbitrary, unique names (completely independent from
the path) and later ask the router to navigate to those names. The router will
automatically create the actual path to navigate to, even inserting required
parameters.

_Named_ navigation has a few advantages over _path-based_ navigation:
- Links can be created without knowing the actual path.
- It is much easier to find all links to a specific route.
- The router knows what links are invalid (and will panic in debug builds).

> When the router encounters an invalid link in a release build, it has to
> handle that problem. You can hook into that process, to display a custom error
> message. See the chapter about
> [named navigation failures](../failures/named.md).

> The router will automatically define the name `""` (empty string) to refer to
> the root index route (`/`).

## Code Example
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;

fn Source(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            // instead of NtPath we use NtName with these parameters:
            // 1. the `name` we want to navigate to
            // 2. a Vec of parameters the router can put in the generated path
            // 3. the query
            target: NtName("target", vec![], QNone),
            "Go to target"
        }
    })
}

fn Target(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Target" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(RcComponent(Source))
            .fixed(
                "target_path",
                Route::new(RcComponent(Target)).name("target")
            )
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus::ssr::render_vdom(&vdom);
# assert_eq!(
#     format!(
#         "<a {attr1} {attr2}>Go to target</a>",
#         attr1 = r#"href="/target_path/" dioxus-prevent-default="onclick""#,
#         attr2 = r#"class="" id="" rel="" target="""#
#     ),
#     html
# )
```
