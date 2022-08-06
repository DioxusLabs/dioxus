use dioxus::prelude::*;
use dioxus_router::{history::MemoryHistory, prelude::*};

use crate::{render, test_routes};

#[cfg(debug_assertions)]
#[test]
#[should_panic = "no route for name \"invalid name\""]
fn panic_in_debug() {
    render(App);

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &||MemoryHistory::with_first(String::from("/named-navigation-failure")),
            }
        })
    }
}

#[cfg(not(debug_assertions))]
#[test]
fn default_content_in_release() {
    let message = format!(
        "<h1>{title}</h1><p>{p1}{p2}<strong>Thank you!</strong></p>",
        title = "A named navigation error has occurred!",
        p1 = "If you see this message, the application you are using has a bug. ",
        p2 = "Please report it to <!--spacer-->the developer so they can fix it."
    );
    assert_eq!(message, render(App));

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                history: &||MemoryHistory::with_first(String::from("/named-navigation-failure")),

                Outlet { }
            }
        })
    }
}
