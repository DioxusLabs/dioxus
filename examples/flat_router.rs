use dioxus::prelude::*;
use dioxus_desktop::{tao::dpi::LogicalSize, Config, WindowBuilder};
use dioxus_router::{Link, Route, Router};

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
    cx.render(rsx! {
        div {
            Router {
                Route { to: "/", "Home" }
                Route { to: "/games", "Games" }
                Route { to: "/play", "Play" }
                Route { to: "/settings", "Settings" }

                p { "----" }
                nav {
                    ul {
                        Link { to: "/", li { "Home" } }
                        Link { to: "/games", li { "Games" } }
                        Link { to: "/play", li { "Play" } }
                        Link { to: "/settings", li { "Settings" } }
                    }
                }
            }
        }
    })
}
