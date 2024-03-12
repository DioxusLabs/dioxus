//! This module provides some utilities around scheduling tasks on the main thread of the browser.
//!
//! The ultimate goal here is to not block the main thread during animation frames, so our animations don't result in "jank".
//!
//! Hence, this module provides Dioxus "Jank Free Rendering" on the web.
//!
//! Because RIC doesn't work on Safari, we polyfill using the "ricpolyfill.js" file and use some basic detection to see
//! if RIC is available.

use futures_util::StreamExt;
use gloo_timers::future::TimeoutFuture;
use js_sys::Function;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{window, Window};

pub(crate) struct RafLoop {
    window: Window,
    ric_receiver: futures_channel::mpsc::UnboundedReceiver<u32>,
    raf_receiver: futures_channel::mpsc::UnboundedReceiver<()>,
    ric_closure: Closure<dyn Fn(JsValue)>,
    raf_closure: Closure<dyn Fn(JsValue)>,
}

impl RafLoop {
    pub fn new() -> Self {
        let (raf_sender, raf_receiver) = futures_channel::mpsc::unbounded();

        let raf_closure: Closure<dyn Fn(JsValue)> = Closure::wrap(Box::new(move |_v: JsValue| {
            raf_sender.unbounded_send(()).unwrap()
        }));

        let (ric_sender, ric_receiver) = futures_channel::mpsc::unbounded();

        let has_idle_callback = {
            let bo = window().unwrap().dyn_into::<js_sys::Object>().unwrap();
            bo.has_own_property(&JsValue::from_str("requestIdleCallback"))
        };
        let ric_closure: Closure<dyn Fn(JsValue)> = Closure::wrap(Box::new(move |v: JsValue| {
            let time_remaining = if has_idle_callback {
                if let Ok(deadline) = v.dyn_into::<web_sys::IdleDeadline>() {
                    deadline.time_remaining() as u32
                } else {
                    10
                }
            } else {
                10
            };

            ric_sender.unbounded_send(time_remaining).unwrap()
        }));

        // execute the polyfill for safari
        Function::new_no_args(include_str!("./ricpolyfill.js"))
            .call0(&JsValue::NULL)
            .unwrap();

        let window = web_sys::window().unwrap();

        Self {
            window,
            raf_receiver,
            raf_closure,
            ric_receiver,
            ric_closure,
        }
    }
    /// waits for some idle time and returns a timeout future that expires after the idle time has passed
    pub async fn wait_for_idle_time(&mut self) -> TimeoutFuture {
        let ric_fn = self.ric_closure.as_ref().dyn_ref::<Function>().unwrap();
        let _cb_id: u32 = self.window.request_idle_callback(ric_fn).unwrap();
        let deadline = self.ric_receiver.next().await.unwrap();
        TimeoutFuture::new(deadline)
    }

    pub async fn wait_for_raf(&mut self) {
        let raf_fn = self.raf_closure.as_ref().dyn_ref::<Function>().unwrap();
        let _id: i32 = self.window.request_animation_frame(raf_fn).unwrap();
        self.raf_receiver.next().await.unwrap();
    }
}
