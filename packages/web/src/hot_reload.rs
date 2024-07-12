//! Handler code for hotreloading.
//!
//! This sets up a websocket connection to the devserver and handles messages from it.
//! There's nto fantastic auto-reconnect logic here - that's coming from the cli devserver.
//! We should look into merging those together.
//!
//! This simply handles the websocket connection and messages from the devserver and brings them
//! back up into the main dioxus loop.

use dioxus_hot_reload::{DevserverMsg, HotReloadMsg};
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use js_sys::JsString;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

pub(crate) fn init() -> UnboundedReceiver<HotReloadMsg> {
    // Create the tx/rx pair that we'll use for the top-level future in the dioxus loop
    let (tx, rx) = unbounded();

    let callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        let Ok(text) = e.data().dyn_into::<JsString>() else {
            return;
        };

        // The devserver messages have some &'static strs in them, so we need to leak the source string
        let string: String = text.into();
        let leaked: &'static str = Box::leak(Box::new(string));

        match serde_json::from_str::<DevserverMsg>(&leaked) {
            Ok(DevserverMsg::HotReload(hr)) => _ = tx.unbounded_send(hr),

            // todo: we want to throw a screen here that shows the user that the devserver has disconnected
            // Would be nice to do that with dioxus itself or some html/css
            // But if the dev server shutsdown we don't want to be super aggressive about it... let's
            // play with other devservers to see how they handle this
            Ok(DevserverMsg::Shutdown) => {
                web_sys::console::error_1(&"Connection to the devserver was closed".into())
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Error parsing devserver message: {}", e).into())
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    // Get the location of the devserver, using the current location plus the /_dioxus path
    // The idea here being that the devserver is always located on the /_dioxus behind a proxy
    let location = web_sys::window().unwrap().location();
    let url = format!(
        "{protocol}//{host}/_dioxus",
        protocol = match location.protocol().unwrap() {
            prot if prot == "https:" => "wss:",
            _ => "ws:",
        },
        host = location.host().unwrap(),
    );

    WebSocket::new(&url)
        .unwrap()
        .set_onmessage(Some(callback.as_ref().unchecked_ref()));

    callback.forget();

    rx
}

/// Force a hotreload of the assets on this page by walking them and changing their URLs to include
/// some extra entropy.
///
/// This should... mostly work.
pub(crate) fn invalidate_browser_asset_cache() {
    // it might be triggering a reload of assets
    // invalidate all the stylesheets on the page
    let links = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector_all("link[rel=stylesheet]")
        .unwrap();

    let noise = js_sys::Math::random();

    for x in 0..links.length() {
        use wasm_bindgen::JsCast;
        let link: web_sys::Element = links.get(x).unwrap().unchecked_into();
        let href = link.get_attribute("href").unwrap();
        _ = link.set_attribute("href", &format!("{}?{}", href, noise));
    }
}
