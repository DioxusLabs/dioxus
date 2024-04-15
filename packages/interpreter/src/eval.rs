/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const NATIVE_EVAL_JS: &str = include_str!("./js/native_eval.js");

#[cfg(feature = "webonly")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct JSOwner {
    _owner: Box<dyn std::any::Any>,
}

#[cfg(feature = "webonly")]
impl JSOwner {
    pub fn new(owner: impl std::any::Any) -> Self {
        Self {
            _owner: Box::new(owner),
        }
    }
}

#[cfg(feature = "webonly")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/eval.js")]
extern "C" {
    pub type DioxusChannel;

    #[wasm_bindgen(constructor)]
    pub fn new(owner: JSOwner) -> DioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &DioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &DioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn send(this: &DioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method)]
    pub async fn recv(this: &DioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn weak(this: &DioxusChannel) -> WeakDioxusChannel;

    pub type WeakDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WeakDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WeakDioxusChannel) -> wasm_bindgen::JsValue;
}
