#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_desktop::tao::dpi::LogicalSize;
use dioxus_router::prelude::*;

fn main() {
    env_logger::init();

    dioxus_desktop::launch_cfg(app, |c| {
        c.with_window(|c| {
            c.with_title("Spinsense Client")
                .with_inner_size(LogicalSize::new(600, 1000))
                .with_resizable(false)
        })
    })
}

fn app(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Home as Component)
            .fixed("games", Games as Component)
            .fixed("play", Play as Component)
            .fixed("settings", Settings as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),

            Outlet { }

            p {
                "----"
            }
            nav {
                ul {
                    Link { target: "/", li { "Home" } }
                    Link { target: "/games", li { "Games" } }
                    Link { target: "/play", li { "Play" } }
                    Link { target: "/settings", li { "Settings" } }
                }
            }

        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        "Home"
    })
}

fn Games(cx: Scope) -> Element {
    cx.render(rsx! {
        "Games"
    })
}

fn Play(cx: Scope) -> Element {
    cx.render(rsx! {
        "Play"
    })
}

fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        "Settings"
    })
}
