use dioxus_core::SchedulerMsg;
use dioxus_core::SetTemplateMsg;
use dioxus_core::VirtualDom;
use futures_channel::mpsc::unbounded;
use futures_channel::mpsc::UnboundedSender;
use futures_util::StreamExt;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{console, MessageEvent, WebSocket};

pub(crate) fn init(dom: &VirtualDom) {
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
    let channel = dom.get_scheduler_channel();

    // change the rsx when new data is received
    let cl = Closure::wrap(Box::new(|e: MessageEvent| {
        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
            let msg: SetTemplateMsg = serde_json::from_str(&format!("{text}")).unwrap();
            channel
                .start_send(SchedulerMsg::SetTemplate(Box::new(msg)))
                .unwrap();
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    ws.set_onmessage(Some(cl.as_ref().unchecked_ref()));
    cl.forget();
}
