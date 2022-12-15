# Programmatic Navigation

Sometimes we want our application to navigate to another page without having the
user click on a link. This is called programmatic navigation.

## Acquiring a [`Navigator`]
To use programmatic navigation, we first have to acquire a [`Navigator`]. For
that purpose we can use the [`use_navigate`] hook.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn Content(cx: Scope) -> Element {
    let nav = use_navigate(&cx).expect("called inside a router");

    // ...
    # unimplemented!()
}
```

## Triggering a Navigation
We can use the [`Navigator`] to trigger four different kinds of navigation:
- `push` will navigate to the target. It works like a regular anchor tag.
- `replace` works like `push`, except that it replaces the current history entry
  instead of adding a new one. This means the prior page cannot be restored with
  the browsers back button.
- `Go back` works like the browsers back button.
- `Go forward` works like the browsers forward button (the opposite of the back
  button).

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn Content(cx: Scope) -> Element {
    let nav = use_navigate(&cx).expect("called inside a router");

    // push
    nav.push("/target");

    // replace
    nav.replace("/target");

    // go back
    nav.go_back();

    // go forward
    nav.go_forward();

    // ...
    # unimplemented!()
}
```

You might have noticed that, like [`Link`], the [`Navigator`]s `push` and
`replace` functions take a [`NavigationTarget`]. This means we can use
[`Internal`], [`Named`] and [`External`].

## External Navigation Targets
Unlike a [`Link`], the [`Navigator`] cannot rely on the browser (or webview) to
handle navigation to external targets via a generated anchor element.

This means, that under certain conditions, navigation to external targets can
fail. See the chapter about
[external navigation failures](../failures/external.md) for more details.

[`External`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.External
[`Internal`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Internal
[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`Named`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Named
[`NavigationTarget`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html
[`Navigator`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/hooks/struct.Navigator.html
[`use_navigate`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_navigate.html
