#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use std::str::FromStr;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_desktop::launch(root);

    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(root);
}

fn root(cx: Scope) -> Element {
    render! {
        Router {}
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
            Outlet {}
        }
    }
}

#[inline_props]
fn Route1(cx: Scope, user_id: usize, dynamic: usize, query: String, extra: String) -> Element {
    render! {
        pre {
            "Route1{{\n\tuser_id:{user_id},\n\tdynamic:{dynamic},\n\tquery:{query},\n\textra:{extra}\n}}"
        }
        Link {
            target: Route::Route1 { user_id: *user_id, dynamic: *dynamic, query: String::new(), extra: extra.clone() + "." },
            "Route1 with extra+\".\""
        }
        p { "Footer" }
        Link {
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
        Link {
            target: Route::Route3 { dynamic: String::new() },
            "Home"
        }
    }
}

#[inline_props]
fn Route3(cx: Scope, dynamic: String) -> Element {
    let navigator = use_navigator(cx);
    let current_route = use_route(cx)?;
    let current_route_str = use_ref(cx, String::new);
    let parsed = Route::from_str(&current_route_str.read());

    let site_map = Route::SITE_MAP
        .iter()
        .flat_map(|seg| seg.flatten().into_iter())
        .collect::<Vec<_>>();

    render! {
        input {
            oninput: move |evt| {
                *current_route_str.write() = evt.value.clone();
            },
            value: "{current_route_str.read()}"
        }
        "dynamic: {dynamic}"
        Link {
            target: Route::Route2 { user_id: 8888 },
            "hello world link"
        }
        button {
            onclick: move |_| { navigator.push(NavigationTarget::External("https://www.google.com".to_string())); },
            "google link"
        }
        p { "Site Map" }
        pre { "{site_map:#?}" }
        p { "Dynamic link" }
        if let Ok(route) = parsed {
            if route != current_route {
                render! {
                    Link {
                        target: route.clone(),
                        "{route}"
                    }
                }
            }
            else {
                None
            }
        }
        else {
            None
        }
    }
}

#[rustfmt::skip]
#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    // Nests with parameters have types taken from child routes
    #[nest("/user/:user_id")]
        // Everything inside the nest has the added parameter `user_id: String`
        // UserFrame is a layout component that will receive the `user_id: String` parameter
        #[layout(UserFrame)]
            // Route1 is a non-layout component that will receive the `user_id: String` and `dynamic: String` parameters
            #[route("/:dynamic?:query")]
            Route1 {
                // The type is taken from the first instance of the dynamic parameter
                user_id: usize,
                dynamic: usize,
                query: String,
                extra: String,
            },
            // Route2 is a non-layout component that will receive the `user_id: String` parameter
            #[route("/hello_world")]
            // You can opt out of the layout by using the `!` prefix
            #[layout(!UserFrame)]
            Route2 { user_id: usize },
        #[end_layout]
    #[end_nest]
    #[redirect("/:id/user", |id: usize| Route::Route3 { dynamic: id.to_string()})]
    #[route("/:dynamic")]
    Route3 { dynamic: String },
}
