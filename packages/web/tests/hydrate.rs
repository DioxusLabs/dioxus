use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::window;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[test]
fn makes_tree() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            div {
                div {
                    h1 {}
                }
                div {
                    h2 {}
                }
            }
        })
    }

    let mut dom = VirtualDom::new(app);
    let muts = dom.rebuild();

    dbg!(muts.edits);
}

#[wasm_bindgen_test]
fn rehydrates() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            div {
                div {
                    h1 { "h1" }
                }
                div {
                    h2 { "h2" }
                }
                button {
                    onclick: move |_| {
                        println!("clicked");
                    },
                    "listener test"
                }
                false.then(|| rsx!{ "hello" })
            }
        })
    }

    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();
    let out = dioxus_ssr::render_vdom_cfg(&dom, |c| c.pre_render(true));

    window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .set_inner_html(&format!("<div id='main'>{}</div>", out));

    dioxus_web::launch_cfg(app, |c| c.hydrate(true));
}
