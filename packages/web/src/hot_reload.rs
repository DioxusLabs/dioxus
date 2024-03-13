#![allow(dead_code)]

use futures_channel::mpsc::UnboundedReceiver;

use dioxus_core::Template;
use web_sys::{console, Element};

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
        console::log_1(&e.clone().into());

        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
            let string: String = text.into();

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&string) {
                // leak the value
                let val: &'static serde_json::Value = Box::leak(Box::new(val));
                let template: Template = Template::deserialize(val).unwrap();
                tx.unbounded_send(template).unwrap();
            } else {
                // it might be triggering a reload of assets
                // invalidate all the stylesheets on the page
                let links = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .query_selector_all("link[rel=stylesheet]")
                    .unwrap();

                console::log_1(&links.clone().into());

                for x in 0..links.length() {
                    console::log_1(&x.clone().into());

                    let link: Element = links.get(x).unwrap().unchecked_into();
                    let href = link.get_attribute("href").unwrap();
                    _ = link.set_attribute("href", &format!("{}?{}", href, js_sys::Math::random()));
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    ws.set_onmessage(Some(cl.as_ref().unchecked_ref()));
    cl.forget();

    rx
}
