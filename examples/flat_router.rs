use dioxus::desktop::{tao::dpi::LogicalSize, Config, WindowBuilder};
use dioxus::prelude::*;
use dioxus::router::prelude::*;

fn main() {
    launch(|| {
        rsx! {
            Router::<Route> {}
        }
    })
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

#[component]
fn Footer() -> Element {
    rsx! {
        Outlet::<Route> {}
        p { "----" }
        nav {
            style { {STYLE} }
            Link { to: Route::Home {}, class: "nav-btn", "Home" }
            Link { to: Route::Games {}, class: "nav-btn", "Games" }
            Link { to: Route::Play {}, class: "nav-btn", "Play" }
            Link { to: Route::Settings {}, class: "nav-btn", "Settings" }
        }
    }
}

#[component]
fn Home() -> Element {
    rsx!("Home")
}

#[component]
fn Games() -> Element {
    rsx!("Games")
}

#[component]
fn Play() -> Element {
    rsx!("Play")
}

#[component]
fn Settings() -> Element {
    rsx!("Settings")
}

const STYLE: &str = r#"
    nav {
        display: flex;
        justify-content: space-around;
    }
    .nav-btn {
        text-decoration: none;
        color: black;
    }
"#;
