# External Navigation

In modern apps, and especially on the web, we often want to send our users to an
other website. [`External`] allows us to make a [`Link`] navigate to an
external page.

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
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| RouterConfiguration {
            # synchronous: true,
            ..Default::default()
        },
        &|| Segment::empty()
    );

    cx.render(rsx! {
        // links need to be inside a router, even if they navigate to an
        // external page
        Link {
            target: NavigationTarget::External("https://dioxuslabs.com/".into()),
            "Go to the dioxus home page"
        }
        Link {
            target: "https://dioxuslabs.com/", // short form
            "Go to the dioxus home page 2"
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(
#     html,
#     format!(
#         "<a {attr1} {attr2}>{text}</a><a {attr1} {attr2}>{text} 2</a>",
#         attr1 = r#"href="https://dioxuslabs.com/" dioxus-prevent-default="""#,
#         attr2 = r#"class="" id="" rel="noopener noreferrer" target="""#,
#         text = "Go to the dioxus home page"
#     )
# )
```

> Note that the short form for an [`ExternalTarget`] looks like the short form
> for an [`InternalTarget`]. The router will create an [`ExternalTarget`] only
> if the URL is absolute.

[`External`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.External
[`Internal`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Internal
[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
