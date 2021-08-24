//! RequestAnimationFrame and RequestIdleCallback port and polyfill.

use std::{cell::RefCell, fmt, rc::Rc};

use gloo_timers::future::TimeoutFuture;
use js_sys::Function;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::{prelude::Closure, JsValue};
use web_sys::Window;

pub struct RafLoop {
    window: Window,
    ric_receiver: async_channel::Receiver<()>,
    raf_receiver: async_channel::Receiver<()>,
    ric_closure: Closure<dyn Fn(JsValue)>,
    raf_closure: Closure<dyn Fn(JsValue)>,
}

impl RafLoop {
    pub fn new() -> Self {
        let (raf_sender, raf_receiver) = async_channel::unbounded();

        let raf_closure: Closure<dyn Fn(JsValue)> =
            Closure::wrap(Box::new(move |v: JsValue| raf_sender.try_send(()).unwrap()));

        let (ric_sender, ric_receiver) = async_channel::unbounded();

        let ric_closure: Closure<dyn Fn(JsValue)> =
            Closure::wrap(Box::new(move |v: JsValue| ric_sender.try_send(()).unwrap()));

        // execute the polyfill for safari
        Function::new_no_args(include_str!("./ricpolyfill.js"))
            .call0(&JsValue::NULL)
            .unwrap();

        Self {
            window: web_sys::window().unwrap(),
            raf_receiver,
            raf_closure,
            ric_receiver,
            ric_closure,
        }
    }
    /// waits for some idle time and returns a timeout future that expires after the idle time has passed
    pub async fn wait_for_idle_time(&self) -> TimeoutFuture {
        // comes with its own safari polyfill :)

        let ric_fn = self.ric_closure.as_ref().dyn_ref::<Function>().unwrap();
        let deadline: u32 = self.window.request_idle_callback(ric_fn).unwrap();

        self.ric_receiver.recv().await.unwrap();

        let deadline = TimeoutFuture::new(deadline);
        deadline
    }

    pub async fn wait_for_raf(&self) {
        let raf_fn = self.raf_closure.as_ref().dyn_ref::<Function>().unwrap();
        let id: i32 = self.window.request_animation_frame(raf_fn).unwrap();
        self.raf_receiver.recv().await.unwrap();
    }
}

#[derive(Debug)]
pub struct AnimationFrame {
    render_id: i32,
    closure: Closure<dyn Fn(JsValue)>,
    callback_wrapper: Rc<RefCell<Option<CallbackWrapper>>>,
}

struct CallbackWrapper(Box<dyn FnOnce(f64) + 'static>);
impl fmt::Debug for CallbackWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CallbackWrapper")
    }
}

impl Drop for AnimationFrame {
    fn drop(&mut self) {
        if self.callback_wrapper.borrow_mut().is_some() {
            web_sys::window()
                .unwrap_throw()
                .cancel_animation_frame(self.render_id)
                .unwrap_throw()
        }
    }
}
