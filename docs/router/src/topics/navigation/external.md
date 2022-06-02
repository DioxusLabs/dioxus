# External Navigation

In modern apps, and especially on the web, we often want to send our users to an
other website. `NtExternal` allows us to make a [`Link`] navigate to an external
page.

> You might already now about
> [external navigation failures](../failures/external.md). The [`Link`]
> component doesn't rely on the code path where those originate. Therefore a
> [`Link`] will never trigger an external navigation failure.

Strictly speaking, a [`Link`] is not necessary for navigating to external
targets, since by definition the router cannot handle them internally. However,
the [`Link`] component is more convenient to use, as it automatically sets the
`rel` attribute for the link, when the target is external.

## Code Example
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;

fn Content(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NtExternal(String::from("https://dioxuslabs.com/")),
            "Go to the dioxus home page"
        }
    })
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            routes: use_segment(&cx, Default::default).clone(),
            # init_only: true,

            // links need to be inside a router, even if they navigate to an
            // external page
            Link {
                target: NtExternal(String::from("https://dioxuslabs.com/")),
                "Go to the dioxus home page"
            }
        }
    })
}

# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus::ssr::render_vdom(&vdom);
# assert_eq!(
#     format!(
#         "<a {attr1} {attr2}>Go to the dioxus home page</a>",
#         attr1 = r#"href="https://dioxuslabs.com/" dioxus-prevent-default="""#,
#         attr2 = r#"class="" id="" rel="noopener noreferrer" target="""#
#     ),
#     html
# )
```

[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
