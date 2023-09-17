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

    dioxus_desktop::launch_cfg(App, cfg)
}

#[component]
fn App(cx: Scope) -> Element {
    render! {
        Router::<Route> {}
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

#[component]
fn Footer(cx: Scope) -> Element {
    render! {
        div {
            Outlet::<Route> { }

            p {
                "----"
            }

            nav {
                ul {
                    li { Link { to: Route::Home {}, "Home" } }
                    li { Link { to: Route::Games {}, "Games" } }
                    li { Link { to: Route::Play {}, "Play" } }
                    li { Link { to: Route::Settings {}, "Settings" } }
                }
            }
        }
    }
}

#[component]
fn Home(cx: Scope) -> Element {
    render!("Home")
}

#[component]
fn Games(cx: Scope) -> Element {
    render!("Games")
}

#[component]
fn Play(cx: Scope) -> Element {
    render!("Play")
}

#[component]
fn Settings(cx: Scope) -> Element {
    render!("Settings")
}
