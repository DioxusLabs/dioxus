use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::DesktopController;
use crate::use_window;
use dioxus_core::ScopeState;
use serde::de::Error;
use serde_json::Value;
use std::future::Future;
use std::future::IntoFuture;
use std::pin::Pin;
use wry::application::dpi::LogicalSize;
use wry::application::event_loop::ControlFlow;
use wry::application::event_loop::EventLoopProxy;
#[cfg(target_os = "ios")]
use wry::application::platform::ios::WindowExtIOS;
use wry::application::window::Fullscreen as WryFullscreen;
use wry::application::window::Window;
use wry::webview::WebView;

/// A future that resolves to the result of a JavaScript evaluation.
pub struct EvalResult {
    pub(crate) reciever: Rc<RefCell<tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>>>,
}

impl IntoFuture for EvalResult {
    type Output = Result<serde_json::Value, serde_json::Error>;

    type IntoFuture = Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let mut reciever = self.reciever.borrow_mut();
            match reciever.recv().await {
                Some(result) => Ok(result),
                None => Err(serde_json::Error::custom("No result returned")),
            }
        }) as Pin<Box<dyn Future<Output = Result<serde_json::Value, serde_json::Error>>>>
    }
}

/// Get a closure that executes any JavaScript in the WebView context.
pub fn use_eval(cx: &ScopeState) -> &Rc<dyn Fn(String) -> EvalResult> {
    let desktop = use_window(cx);
    &*cx.use_hook(|| {
        let desktop = desktop.clone();

        Rc::new(move |script: String| desktop.eval(&script)) as Rc<dyn Fn(String) -> EvalResult>
    })
}
