use crate::DesktopService;
use dioxus_document::{Document, Eval, EvalError};
use std::sync::{Arc, Mutex};

impl Document for DesktopService {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn set_title(&self, title: String) {
        self.window.set_title(&title);
    }

    fn eval(&self, js: String) -> Eval {
        let (tx, eval) = Eval::from_parts();

        // Dumb wry has a signature of Fn instead of FnOnce, meaning we need to put the callback in a closure
        // that uses rwlock + option to make sure we don't run the callback twice
        let tx = Arc::new(Mutex::new(Some(tx)));
        let callback = {
            let tx = tx.clone();
            move |res| {
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(Ok(res));
                }
            }
        };

        let res = self.webview.evaluate_script_with_callback(&js, callback);
        if let Err(err) = res {
            _ = tx
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(Err(EvalError::Communication(err.to_string())));
        }

        eval
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: Vec<(&str, String)>,
        contents: Option<String>,
    ) {
        let contents = contents.unwrap_or_default();
        let attr_iter = attributes
            .into_iter()
            .map(|(name, value)| format!(r#"element.setAttribute("{name}", "{value}");"#))
            .collect::<Vec<_>>()
            .join("");

        self.eval(format!(
            r#"
            let element = document.createElement("{name}");
            {attr_iter}
            element.innerHTML = "{contents}";
            document.head.appendChild(element);
            "#,
        ));
    }
}
