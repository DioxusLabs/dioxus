#![allow(dead_code)]

use futures_channel::mpsc::UnboundedReceiver;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

#[cfg(not(debug_assertions))]
pub(crate) fn init() -> UnboundedReceiver<String> {
    let (tx, rx) = futures_channel::mpsc::unbounded();

    std::mem::forget(tx);

    rx
}

#[cfg(debug_assertions)]
pub(crate) fn init() -> UnboundedReceiver<String> {
    use std::convert::TryInto;

    let window = web_sys::window().unwrap();

    let protocol = if window.location().protocol().unwrap() == "https:" {
        "wss:"
    } else {
        "ws:"
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
            if let Ok(val) = text.try_into() {
                _ = tx.unbounded_send(val);
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    ws.set_onmessage(Some(cl.as_ref().unchecked_ref()));
    cl.forget();

    rx
}
