// Regression test for https://github.com/DioxusLabs/dioxus/issues/4440
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

#[used]
static BINDINGS_JS: Asset = asset!(
    "/assets/bindings.js",
    AssetOptions::js().with_hash_suffix(false)
);

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_effect(|| {
        Foo::new();
    });
    rsx! {}
}

#[wasm_bindgen(raw_module = "/assets/bindings.js")]
extern "C" {
    #[wasm_bindgen]
    pub type Foo;
    #[wasm_bindgen(constructor)]
    pub fn new() -> Foo;
}
