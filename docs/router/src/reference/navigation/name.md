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

> The router will automatically define the name [`RootIndex`] to refer to the
> root index route (`/`).
>
> It will also add other names (all of them are in the prelude module) in
> certain conditions. None of these names can be used for app defined routes.

## Code Example

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

// we define a unit struct which will serve as our name
struct TargetName;

fn Source(cx: Scope) -> Element {
    render! {
        Link {
            // instead of InternalTarget we use NamedTarget (via the `named` fn)
            // we can use the returned value to add parameters or a query
            target: named::<TargetName>().query("query"),
            "Go to target"
        }
    }
}

fn Target(cx: Scope) -> Element {
    render! {
        h1 { "Target" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            ..Default::default()
        },
        &|| {
            Segment::content(comp(Source))
                .fixed(
                    "target_path",
                    Route::content(comp(Target)).name::<TargetName>()
                )
        }
    );

    render! {
        Outlet { }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(
#     html,
#     format!(
#         "<a {attr1} {attr2}>Go to target</a>",
#         attr1 = r#"href="/target_path?query" dioxus-prevent-default="onclick""#,
#         attr2 = r#"class="" id="" rel="" target="""#
#     )
# )
```

## Check if a name is present

You can check if a specific name is present for the current route. This works
similar to getting the value of a [parameter route](../routes/parameter.md) and
the same restrictions apply.

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
struct SomeName;

fn Content(cx: Scope) -> Element {
    let route = use_route(cx).expect("needs to be in router");

    if route.is_at(&named::<SomeName>(), false) {
        // do something
    }

    // ...
    # todo!()
}
```

[`RootIndex`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/prelude/struct.RootIndex.html
