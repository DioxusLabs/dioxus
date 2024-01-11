use dioxus::prelude::*;
use dioxus_web::Config;
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
                {false.then(|| rsx!{ "hello" })}
            }
        })
    }

    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();
    let out = dioxus_ssr::render(&dom);

    window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .set_inner_html(&format!("<div id='main'>{out}</div>"));

    dioxus_web::launch_cfg(app, Config::new().hydrate(true));
}
