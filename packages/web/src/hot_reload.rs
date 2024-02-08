#![allow(dead_code)]

use futures_channel::mpsc::UnboundedReceiver;

use dioxus_core::Template;

pub(crate) fn init() -> UnboundedReceiver<Template> {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{MessageEvent, WebSocket};

    use serde::Deserialize;

    let window = web_sys::window().unwrap();

    let protocol = match window.location().protocol().unwrap() {
        prot if prot == "https:" => "wss:",
        _ => "ws:",
    };

    let url = format!(
        "{protocol}//{}/_dioxus/hot_reload",
        window.location().host().unwrap()
    );

    let ws = WebSocket::new(&url).unwrap();

    let (tx, rx) = futures_channel::mpsc::unbounded();

    // change the rsx when new data is received
    let cl = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
            let string: String = text.into();
            let val = serde_json::from_str::<serde_json::Value>(&string).unwrap();
            // leak the value
            let val: &'static serde_json::Value = Box::leak(Box::new(val));
            let template: Template = Template::deserialize(val).unwrap();
            tx.unbounded_send(template).unwrap();
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    ws.set_onmessage(Some(cl.as_ref().unchecked_ref()));
    cl.forget();

    rx
}
