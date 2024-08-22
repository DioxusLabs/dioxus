use dioxus::prelude::*;
use dioxus_web::Config;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::window;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[test]
fn makes_tree() {
    fn app() -> Element {
        rsx! {
            div {
                div { h1 {} }
                div { h2 {} }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let muts = dom.rebuild_to_vec();

    dbg!(muts.edits);
}

#[wasm_bindgen_test]
fn rehydrates() {
    fn app() -> Element {
        rsx! {
            div {
                div { h1 { "h1" } }
                div { h2 { "h2" } }
                button {
                    onclick: move |_| {
                        println!("clicked");
                    },
                    "listener test"
                }
                {false.then(|| rsx! { "hello" })}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);
    let out = dioxus_ssr::render(&dom);

    window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .set_inner_html(&format!("<div id='main'>{out}</div>"));

    dioxus_web::launch::launch_cfg(app, Config::new().hydrate(true));
}
