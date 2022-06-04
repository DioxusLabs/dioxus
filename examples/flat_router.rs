use dioxus::desktop::tao::dpi::LogicalSize;
use dioxus::prelude::*;

fn main() {
    env_logger::init();

    dioxus::desktop::launch_cfg(app, |c| {
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
                    Link { target: "/".into(), li { "Home" } }
                    Link { target: "/games".into(), li { "Games" } }
                    Link { target: "/play".into(), li { "Play" } }
                    Link { target: "/settings".into(), li { "Settings" } }
                }
            }

        }
    })
}

#[allow(non_snake_case)]
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        "Home"
    })
}

#[allow(non_snake_case)]
fn Games(cx: Scope) -> Element {
    cx.render(rsx! {
        "Games"
    })
}

#[allow(non_snake_case)]
fn Play(cx: Scope) -> Element {
    cx.render(rsx! {
        "Play"
    })
}

#[allow(non_snake_case)]
fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        "Settings"
    })
}
