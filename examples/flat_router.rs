use dioxus::prelude::*;
use dioxus::desktop::tao::dpi::LogicalSize;

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
    cx.render(rsx! {
        Router {
            Route { to: "/", "Home" }
            Route { to: "/games", "Games" }
            Route { to: "/play", "Play" }
            Route { to: "/settings", "Settings" }

            p {
                "----"
            }
            nav {
                ul {
                    Link { to: "/", li { "Home" } }
                    Link { to: "/games", li { "Games" } }
                    Link { to: "/play", li { "Play" } }
                    Link { to: "/settings", li { "Settings" } }
                }
            }
        }
    })
}
