/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const EVAL_JS: &str = include_str!("./js/eval.js");


#[cfg(feature = "webonly")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    pub type DioxusChannel;

    #[wasm_bindgen(constructor)]
    pub fn new() -> DioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &DioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &DioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn send(this: &DioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method)]
    pub async fn recv(this: &DioxusChannel) -> wasm_bindgen::JsValue;
}
