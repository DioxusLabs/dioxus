use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::*;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    dioxus_web::launch(App, |c| c);
}

static App: Component<()> = |cx, props| {
    #[derive(Clone, Debug, PartialEq)]
    enum Route {
        Home,
        About,
        NotFound,
    }

    let route = use_router(cx, |s| match s {
        "/" => Route::Home,
        "/about" => Route::About,
        _ => Route::NotFound,
    });

    cx.render(rsx! {
        div {
            {match route {
                Route::Home => rsx!(h1 { "Home" }),
                Route::About => rsx!(h1 { "About" }),
                Route::NotFound => rsx!(h1 { "NotFound" }),
            }}
            nav {
                Link { to: Route::Home, href: |_| "/".to_string() }
                Link { to: Route::About, href: |_| "/about".to_string() }
            }
        }
    })
};
