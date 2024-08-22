use std::sync::{Arc, Mutex};

use crate::DesktopContext;
use dioxus_document::{Document, Eval, EvalError};

/// Represents the desktop-target's provider of evaluators.
pub struct DesktopDocument {
    pub(crate) cx: DesktopContext,
}

impl DesktopDocument {
    pub fn new(desktop_ctx: DesktopContext) -> Self {
        Self { cx: desktop_ctx }
    }
}

impl Document for DesktopDocument {
    fn eval(&self, js: String) -> Eval {
        let (tx, eval) = Eval::from_parts();

        // Dumb wry has a signature of Fn instead of FnOnce, meaning we need to put the callback in a closure
        // that uses rwlock + option to make sure we don't run the callback twice
        let _tx = Arc::new(Mutex::new(Some(tx)));
        let tx = _tx.clone();
        let callback = move |res| {
            if let Some(tx) = tx.lock().unwrap().take() {
                let _ = tx.send(Ok(res));
            }
        };

        let res = self.cx.webview.evaluate_script_with_callback(&js, callback);
        if let Err(err) = res {
            _ = _tx
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(Err(EvalError::Communication(err.to_string())));
        }

        eval
    }

    fn set_title(&self, title: String) {
        self.cx.window.set_title(&title);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    ) {
        todo!("create the head element stuff")
    }
}
