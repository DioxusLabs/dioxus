//! Handler code for hotreloading.
//!
//! This sets up a websocket connection to the devserver and handles messages from it.
//! We also set up a little recursive timer that will attempt to reconnect if the connection is lost.

use std::fmt::Display;
use std::time::Duration;

use dioxus_core::ScopeId;
use dioxus_devtools::{DevserverMsg, HotReloadMsg};
use dioxus_document::eval;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use js_sys::JsString;
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
use web_sys::{window, Event, MessageEvent, WebSocket};

const POLL_INTERVAL_MIN: i32 = 250;
const POLL_INTERVAL_MAX: i32 = 4000;
const POLL_INTERVAL_SCALE_FACTOR: i32 = 2;

/// Amount of time that toats should be displayed.
const TOAST_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn init() -> UnboundedReceiver<HotReloadMsg> {
    // Create the tx/rx pair that we'll use for the top-level future in the dioxus loop
    let (tx, rx) = unbounded();

    // Wire up the websocket to the devserver
    make_ws(tx, POLL_INTERVAL_MIN, false);

    rx
}

fn make_ws(tx: UnboundedSender<HotReloadMsg>, poll_interval: i32, reload: bool) {
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

    let ws = WebSocket::new(&url).unwrap();

    // Set the onmessage handler to bounce messages off to the main dioxus loop
    let tx_ = tx.clone();
    ws.set_onmessage(Some(
        Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let Ok(text) = e.data().dyn_into::<JsString>() else {
                return;
            };

            // The devserver messages have some &'static strs in them, so we need to leak the source string
            let string: String = text.into();
            // let leaked: &'static str = Box::leak(Box::new(string));

            match serde_json::from_str::<DevserverMsg>(&string) {
                Ok(DevserverMsg::HotReload(hr)) => _ = tx_.unbounded_send(hr),

                // todo: we want to throw a screen here that shows the user that the devserver has disconnected
                // Would be nice to do that with dioxus itself or some html/css
                // But if the dev server shutsdown we don't want to be super aggressive about it... let's
                // play with other devservers to see how they handle this
                Ok(DevserverMsg::Shutdown) => {
                    web_sys::console::error_1(&"Connection to the devserver was closed".into())
                }

                // The devserver is telling us that it started a full rebuild. This does not mean that it is ready.
                Ok(DevserverMsg::FullReloadStart) => show_toast(
                    "Your app is being rebuilt.",
                    "A non-hot-reloadable change occurred and we must rebuild.",
                    ToastLevel::Info,
                    Duration::from_secs(600),
                    false,
                ),
                // The devserver is telling us that the full rebuild failed.
                Ok(DevserverMsg::FullReloadFailed) => show_toast(
                    "Oops! The build failed.",
                    "We tried to rebuild your app, but something went wrong.",
                    ToastLevel::Error,
                    TOAST_TIMEOUT,
                    false,
                ),

                // The devserver is telling us to reload the whole page
                Ok(DevserverMsg::FullReloadCommand) => {
                    show_toast(
                        "Successfully rebuilt.",
                        "Your app was rebuilt successfully and without error.",
                        ToastLevel::Success,
                        TOAST_TIMEOUT,
                        true,
                    );
                    window().unwrap().location().reload().unwrap()
                }

                Err(e) => web_sys::console::error_1(
                    &format!("Error parsing devserver message: {}", e).into(),
                ),
            }
        })
        .into_js_value()
        .as_ref()
        .unchecked_ref(),
    ));

    // Set the onclose handler to reload the page if the connection is closed
    ws.set_onclose(Some(
        Closure::<dyn FnMut(Event)>::new(move |e: Event| {
            // Firefox will send a 1001 code when the connection is closed because the page is reloaded
            // Only firefox will trigger the onclose event when the page is reloaded manually: https://stackoverflow.com/questions/10965720/should-websocket-onclose-be-triggered-by-user-navigation-or-refresh
            // We should not reload the page in this case
            if js_sys::Reflect::get(&e, &"code".into()).map(|f| f.as_f64().unwrap_or(0.0))
                == Ok(1001.0)
            {
                return;
            }

            // set timeout to reload the page in timeout_ms
            let tx = tx.clone();
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    Closure::<dyn FnMut()>::new(move || {
                        make_ws(
                            tx.clone(),
                            POLL_INTERVAL_MAX.min(poll_interval * POLL_INTERVAL_SCALE_FACTOR),
                            true,
                        );
                    })
                    .into_js_value()
                    .as_ref()
                    .unchecked_ref(),
                    poll_interval,
                )
                .unwrap();
        })
        .into_js_value()
        .as_ref()
        .unchecked_ref(),
    ));

    // Set the onopen handler to reload the page if the connection is closed
    ws.set_onopen(Some(
        Closure::<dyn FnMut(MessageEvent)>::new(move |_evt| {
            if reload {
                window().unwrap().location().reload().unwrap()
            }
        })
        .into_js_value()
        .as_ref()
        .unchecked_ref(),
    ));

    // monkey patch our console.log / console.error to send the logs to the websocket
    // this will let us see the logs in the devserver!
    // We only do this if we're not reloading the page, since that will cause duplicate monkey patches
    if !reload {
        // the method we need to patch:
        // https://developer.mozilla.org/en-US/docs/Web/API/Console/log
        // log, info, warn, error, debug
        let ws: &JsValue = ws.as_ref();
        dioxus_interpreter_js::minimal_bindings::monkeyPatchConsole(ws.clone());
    }
}

/// Represents what color the toast should have.
enum ToastLevel {
    /// Green
    Success,
    /// Blue
    Info,
    /// Red
    Error,
}

impl Display for ToastLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToastLevel::Success => write!(f, "success"),
            ToastLevel::Info => write!(f, "info"),
            ToastLevel::Error => write!(f, "error"),
        }
    }
}

/// Displays a toast to the developer.
fn show_toast(
    header_text: &str,
    message: &str,
    level: ToastLevel,
    duration: Duration,
    after_reload: bool,
) {
    let as_ms = duration.as_millis();

    let js_fn_name = match after_reload {
        true => "scheduleDXToast",
        false => "showDXToast",
    };

    ScopeId::ROOT.in_runtime(|| {
        eval(&format!(
            r#"
            if (typeof {js_fn_name} !== "undefined") {{
                {js_fn_name}("{header_text}", "{message}", "{level}", {as_ms});
            }}
            "#,
        ));
    });
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
