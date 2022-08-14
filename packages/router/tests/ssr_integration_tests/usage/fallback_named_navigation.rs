#[cfg(not(debug_assertions))]
use std::any::TypeId;

use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::{render, test_routes};

#[cfg(debug_assertions)]
#[test]
#[should_panic = "no route for name \"&str\""]
fn panic_in_debug() {
    render(App);

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                initial_path: "/named-navigation-failure",
            }
        })
    }
}

#[cfg(not(debug_assertions))]
#[test]
fn default_content_in_release() {
    let message = format!(
        "<h1>{title}</h1><p>{p1}{p2}<strong>Thank you!</strong></p><!--placeholder-->",
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
                initial_path: "/named-navigation-failure",

                Outlet { }
                Content { }
            }
        })
    }

    #[allow(non_snake_case)]
    fn Content(cx: Scope) -> Element {
        let route = use_route(&cx).expect("in router");

        assert_eq!(route.names.len(), 2);
        assert!(route.names.contains(&TypeId::of::<RootIndex>()));
        assert!(route
            .names
            .contains(&TypeId::of::<FallbackNamedNavigation>()));

        None
    }
}
