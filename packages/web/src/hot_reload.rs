#![allow(dead_code)]

use dioxus_hot_reload::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::UnboundedReceiver;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

pub(crate) fn init() -> UnboundedReceiver<HotReloadMsg> {
    let window = web_sys::window().unwrap();

    let url = format!(
        "{protocol}//{host}/_dioxus",
        protocol = match window.location().protocol().unwrap() {
            prot if prot == "https:" => "wss:",
            _ => "ws:",
        },
        host = window.location().host().unwrap(),
    );

    let ws = WebSocket::new(&url).unwrap();
    let (tx, rx) = futures_channel::mpsc::unbounded();

    // change the rsx when new data is received
    let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        let Ok(text) = e.data().dyn_into::<js_sys::JsString>() else {
            return;
        };

        let string: String = text.into();
        let leaked: &'static str = Box::leak(Box::new(string));

        let Ok(msg) = serde_json::from_str::<DevserverMsg>(&leaked) else {
            return;
        };

        match msg {
            DevserverMsg::HotReload(hr) => _ = tx.unbounded_send(hr),
            DevserverMsg::Shutdown => {}
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    ws.set_onmessage(Some(callback.as_ref().unchecked_ref()));

    callback.forget();

    rx
}
