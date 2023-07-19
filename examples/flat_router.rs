#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_desktop::{tao::dpi::LogicalSize, Config, WindowBuilder};
use dioxus_router::prelude::*;

fn main() {
    env_logger::init();

    let cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(600, 1000))
            .with_resizable(false),
    );

    dioxus_desktop::launch_cfg(app, cfg)
}

fn app(cx: Scope) -> Element {
    render! {
        Router {}
    }
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[layout(Footer)]
        #[route("/")]
        Home {},
        #[route("/games")]
        Games {},
        #[route("/play")]
        Play {},
        #[route("/settings")]
        Settings {},
}

#[inline_props]
fn Footer(cx: Scope) -> Element {
    render! {
        div {
            Outlet { }

            p {
                "----"
            }

            nav {
                ul {
                    li { Link { target: Route::Home {}, "Home" } }
                    li { Link { target: Route::Games {}, "Games" } }
                    li { Link { target: Route::Play {}, "Play" } }
                    li { Link { target: Route::Settings {}, "Settings" } }
                }
            }
        }
    }
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    render!("Home")
}

#[inline_props]
fn Games(cx: Scope) -> Element {
    render!("Games")
}

#[inline_props]
fn Play(cx: Scope) -> Element {
    render!("Play")
}

#[inline_props]
fn Settings(cx: Scope) -> Element {
    render!("Settings")
}
