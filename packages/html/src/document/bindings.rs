/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
#[cfg(feature = "native-bind")]
pub const NATIVE_EVAL_JS: &str = include_str!("../js/native_eval.js");

#[cfg(feature = "wasm-bind")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct JSOwner {
    _owner: Box<dyn std::any::Any>,
}

#[cfg(feature = "wasm-bind")]
impl JSOwner {
    pub fn new(owner: impl std::any::Any) -> Self {
        Self {
            _owner: Box::new(owner),
        }
    }
}

#[cfg(feature = "wasm-bind")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/js/eval.js")]
extern "C" {
    pub type WebDioxusChannel;

    #[wasm_bindgen(constructor)]
    pub fn new(owner: JSOwner) -> WebDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn send(this: &WebDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method)]
    pub async fn recv(this: &WebDioxusChannel) -> wasm_bindgen::JsValue;

    #[wasm_bindgen(method)]
    pub fn weak(this: &WebDioxusChannel) -> WeakDioxusChannel;

    pub type WeakDioxusChannel;

    #[wasm_bindgen(method, js_name = "rustSend")]
    pub fn rust_send(this: &WeakDioxusChannel, value: wasm_bindgen::JsValue);

    #[wasm_bindgen(method, js_name = "rustRecv")]
    pub async fn rust_recv(this: &WeakDioxusChannel) -> wasm_bindgen::JsValue;
}
