use dioxus::prelude::*;
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
            Bapp {}
        }
        (0..10).map(|f| rsx!{
            div {
                "thing {f}"
            }
        })
    })
}

fn Bapp(cx: Scope) -> Element {
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
    log::trace!("{}", pre);

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
