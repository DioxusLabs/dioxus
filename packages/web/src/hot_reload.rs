use dioxus_core::VirtualDom;
use dioxus_rsx_interpreter::error::Error;
use dioxus_rsx_interpreter::{ErrorHandler, SetManyRsxMessage, RSX_CONTEXT};
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

    // change the rsx when new data is received
    let cl = Closure::wrap(Box::new(|e: MessageEvent| {
        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
            let msgs: SetManyRsxMessage = serde_json::from_str(&format!("{text}")).unwrap();
            RSX_CONTEXT.extend(msgs);
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    ws.set_onmessage(Some(cl.as_ref().unchecked_ref()));
    cl.forget();

    let (error_channel_sender, mut error_channel_receiver) = unbounded();

    struct WebErrorHandler {
        sender: UnboundedSender<Error>,
    }

    impl ErrorHandler for WebErrorHandler {
        fn handle_error(&self, err: dioxus_rsx_interpreter::error::Error) {
            self.sender.unbounded_send(err).unwrap();
        }
    }

    RSX_CONTEXT.set_error_handler(WebErrorHandler {
        sender: error_channel_sender,
    });

    RSX_CONTEXT.provide_scheduler_channel(dom.get_scheduler_channel());

    // forward stream to the websocket
    dom.base_scope().spawn_forever(async move {
        while let Some(err) = error_channel_receiver.next().await {
            if ws.ready_state() == WebSocket::OPEN {
                ws.send_with_str(serde_json::to_string(&err).unwrap().as_str())
                    .unwrap();
            } else {
                console::warn_1(&"WebSocket is not open, cannot send error. Run with dioxus serve --hot-reload to enable hot reloading.".into());
                panic!("{}", err);
            }
        }
    });
}
