use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::*;
use serde::{Deserialize, Serialize};

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    dioxus_web::launch(APP);
}

static APP: Component<()> = |cx| {
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    enum Route {
        Home,
        About,
        NotFound,
    }
    impl Default for Route {
        fn default() -> Self {
            Route::Home
        }
    }

    let route = use_router(&cx, |c| {});

    cx.render(rsx! {
        div {
            {match route {
                Route::Home => rsx!(h1 { "Home" }),
                Route::About => rsx!(h1 { "About" }),
                Route::NotFound => rsx!(h1 { "NotFound" }),
            }}
            nav {
                Link { to: Route::Home, href: "/" }
                Link { to: Route::About, href: "/about" }
            }
        }
    })
};
