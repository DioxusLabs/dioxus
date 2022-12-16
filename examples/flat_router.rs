#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_desktop::{tao::dpi::LogicalSize, Config, WindowBuilder};
use dioxus_router::prelude::*;

fn main() {
    env_logger::init();

    let cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("Spinsense Client")
            .with_inner_size(LogicalSize::new(600, 1000))
            .with_resizable(false),
    );

    dioxus_desktop::launch_cfg(app, cfg)
}

fn app(cx: Scope) -> Element {
    use_router(cx, &|| RouterConfiguration::default(), &|| {
        Segment::content(comp(Home))
            .fixed("games", comp(Games))
            .fixed("play", comp(Play))
            .fixed("settings", comp(Settings))
    });

    render! {
        Outlet { }

        p {
            "----"
        }

        nav {
            ul {
                li { Link { target: "/", "Home" } }
                li { Link { target: "/games", "Games" } }
                li { Link { target: "/play", "Play" } }
                li { Link { target: "/settings", "Settings" } }
            }
        }
    }
}

fn Home(cx: Scope) -> Element {
    render!("Home")
}

fn Games(cx: Scope) -> Element {
    render!("Games")
}

fn Play(cx: Scope) -> Element {
    render!("Play")
}

fn Settings(cx: Scope) -> Element {
    render!("Settings")
}
