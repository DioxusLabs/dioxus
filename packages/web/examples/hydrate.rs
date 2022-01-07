use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::window;

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "thing 1" }
        }
        div {
            h2 { "thing 2"}
        }
        div {
            h2 { "thing 2"}
            "asd"
            "asd"
            bapp()
        }
        (0..10).map(|f| rsx!{
            div {
                "thing {f}"
            }
        })
    })
}

fn bapp(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "thing 1" }
        }
        div {
            h2 { "thing 2"}
        }
        div {
            h2 { "thing 2"}
            "asd"
            "asd"
        }
    })
}

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));

    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();

    let pre = dioxus_ssr::pre_render_vdom(&dom);
    log::debug!("{}", pre);

    // set the inner content of main to the pre-rendered content
    window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("main")
        .unwrap()
        .set_inner_html(&pre);

    // now rehydtrate
    dioxus_web::launch_with_props(app, (), |c| c.hydrate(true));
}
