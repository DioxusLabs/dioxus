use dioxus_html::document::{Document, EvalError, Evaluator};
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

use crate::{query::Query, DesktopContext};

/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const NATIVE_EVAL_JS: &str = include_str!("./js/native_eval.js");

/// Represents the desktop-target's provider of evaluators.
pub struct DesktopDocument {
    pub(crate) desktop_ctx: DesktopContext,
}

impl DesktopDocument {
    pub fn new(desktop_ctx: DesktopContext) -> Self {
        Self { desktop_ctx }
    }
}

impl Document for DesktopDocument {
    fn new_evaluator(&self, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        DesktopEvaluator::create(self.desktop_ctx.clone(), js)
    }

impl Document for DesktopService {
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
            move |res: String| {
                if let Ok(res) = serde_json::from_str(&res) {
                    if let Some(tx) = tx.lock().unwrap().take() {
                        let _ = tx.send(Ok(res));
                    }
                } else {
                    tracing::error!("Failed to deserialize eval result: {res:?}");
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

    fn current_route(&self) -> String {
        todo!()
    }

    fn go_back(&self) {
        todo!()
    }

    fn go_forward(&self) {
        todo!()
    }

    fn push_route(&self, route: String) {
        todo!()
    }

    fn replace_route(&self, path: String) {
        todo!()
    }
}
