use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[test]
fn makes_tree() {
    fn app(cx: Scope) -> Element {
        cx.render(rsx! {
            div {
                h1 {}
            }
            div {
                h2 {}
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
                h1 {}
            }
            div {
                h2 {}
            }
        })
    }

    dioxus_web::launch(app);
}
