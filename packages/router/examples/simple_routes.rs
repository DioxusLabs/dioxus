#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_desktop::launch(root);

    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(root);
}

fn root(cx: Scope) -> Element {
    render! {
        Router::<Route> {
            config: RouterConfiguration {
                history: {
                    #[cfg(not(target_arch = "wasm32"))]
                    let history = Box::<MemoryHistory::<Route>>::default();
                    #[cfg(target_arch = "wasm32")]
                    let history = Box::<WebHistory::<Route>>::default();
                    history
                },
                ..Default::default()
            }
        }
    }
}

#[inline_props]
fn UserFrame(cx: Scope, user_id: usize) -> Element {
    render! {
        pre {
            "UserFrame{{\n\tuser_id:{user_id}\n}}"
        }
        div {
            background_color: "rgba(0,0,0,50%)",
            "children:"
            Outlet::<Route> {}
        }
    }
}

#[inline_props]
fn Route1(cx: Scope, user_id: usize, dynamic: usize, extra: String) -> Element {
    render! {
        pre {
            "Route1{{\n\tuser_id:{user_id},\n\tdynamic:{dynamic},\n\textra:{extra}\n}}"
        }
        Link::<Route> {
            target: Route::Route1 { user_id: *user_id, dynamic: *dynamic, extra: extra.clone() + "." },
            "Route1 with extra+\".\""
        }
        p { "Footer" }
        Link::<Route> {
            target: Route::Route3 { dynamic: String::new() },
            "Home"
        }
    }
}

#[inline_props]
fn Route2(cx: Scope, user_id: usize) -> Element {
    render! {
        pre {
            "Route2{{\n\tuser_id:{user_id}\n}}"
        }
        (0..*user_id).map(|i| rsx!{ p { "{i}" } }),
        p { "Footer" }
        Link::<Route> {
            target: Route::Route3 { dynamic: String::new() },
            "Home"
        }
    }
}

#[inline_props]
fn Route3(cx: Scope, dynamic: String) -> Element {
    let router = use_router(cx);
    let router_route = router.current();
    let current_route = use_ref(cx, String::new);
    let parsed = Route::from_str(&current_route.read());

    let site_map = Route::SITE_MAP
        .iter()
        .flat_map(|seg| seg.flatten().into_iter())
        .collect::<Vec<_>>();

    render! {
        input {
            oninput: move |evt| {
                *current_route.write() = evt.value.clone();
            },
            value: "{current_route.read()}"
        }
        "dynamic: {dynamic}"
        Link::<Route> {
            target: Route::Route2 { user_id: 8888 },
            "hello world link"
        }
        p { "Site Map" }
        pre { "{site_map:#?}" }
        p { "Dynamic link" }
        if let Ok(route) = parsed {
            if route != router_route {
                render! {
                    Link::<Route> {
                        target: route.clone(),
                        "{route}"
                    }
                }
            }
            else{None}
        }
        else{None}
    }
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Routable)]
enum Route {
    // Nests with parameters have types taken from child routes
    #[nest("/user/:user_id")]
        // Everything inside the nest has the added parameter `user_id: String`
        // UserFrame is a layout component that will receive the `user_id: String` parameter
        #[layout(UserFrame)]
        // Route1 is a non-layout component that will receive the `user_id: String` and `dynamic: String` parameters
        #[route("/:dynamic", Route1)]
            Route1 {
                // The type is taken from the first instance of the dynamic parameter
                user_id: usize,
                dynamic: usize,
                extra: String,
            },
            // Route2 is a non-layout component that will receive the `user_id: String` parameter
            #[route("/hello_world", Route2)]
            // You can opt out of the layout by using the `!` prefix
            #[layout(!UserFrame)]
            Route2 { user_id: usize },
        #[end_layout]
    #[end_nest]
    #[route("/:dynamic", Route3)]
    Route3 { dynamic: String },
}
