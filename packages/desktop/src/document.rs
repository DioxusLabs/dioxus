use crate::{query::Query, DesktopContext, WeakDesktopContext};
use dioxus_core::prelude::queue_effect;
use dioxus_document::{
    create_element_in_head, Document, Eval, EvalError, Evaluator, LinkProps, MetaProps,
    ScriptProps, StyleProps,
};

use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const NATIVE_EVAL_JS: &str = include_str!("./js/native_eval.js");

/// Represents the desktop-target's provider of evaluators.
#[derive(Clone)]
pub struct DesktopDocument {
    pub(crate) desktop_ctx: WeakDesktopContext,
}

impl DesktopDocument {
    pub fn new(desktop_ctx: DesktopContext) -> Self {
        let desktop_ctx = std::rc::Rc::downgrade(&desktop_ctx);
        Self { desktop_ctx }
    }
}

impl Document for DesktopDocument {
    fn eval(&self, js: String) -> Eval {
        Eval::new(DesktopEvaluator::create(
            self.desktop_ctx
                .upgrade()
                .expect("Window to exist when document is alive"),
            js,
        ))
    }

    fn set_title(&self, title: String) {
        if let Some(ctx) = self.desktop_ctx.upgrade() {
            ctx.set_title(&title);
        }
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("meta", &props.attributes(), None));
        });
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "script",
                &props.attributes(),
                props.script_contents().ok(),
            ));
        });
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head(
                "style",
                &props.attributes(),
                props.style_contents().ok(),
            ));
        });
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(create_element_in_head("link", &props.attributes(), None));
        });
    }
}

/// Represents a desktop-target's JavaScript evaluator.
pub(crate) struct DesktopEvaluator {
    query: Query<serde_json::Value>,
}

impl DesktopEvaluator {
    /// Creates a new evaluator for desktop-based targets.
    pub fn create(desktop_ctx: DesktopContext, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let query = desktop_ctx.query.new_query(&js, desktop_ctx.clone());

        // We create a generational box that is owned by the query slot so that when we drop the query slot, the generational box is also dropped.
        let owner = UnsyncStorage::owner();
        let query_id = query.id;
        let query = owner.insert(Box::new(DesktopEvaluator { query }) as Box<dyn Evaluator>);
        desktop_ctx.query.active_requests.slab.borrow_mut()[query_id].owner = Some(owner);

        query
    }
}

impl Evaluator for DesktopEvaluator {
    fn poll_join(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_result(cx)
            .map_err(|e| EvalError::Communication(e.to_string()))
    }

    /// Sends a message to the evaluated JavaScript.
    fn send(&self, data: serde_json::Value) -> Result<(), EvalError> {
        if let Err(e) = self.query.send(data) {
            return Err(EvalError::Communication(e.to_string()));
        }
        Ok(())
    }

    /// Gets an UnboundedReceiver to receive messages from the evaluated JavaScript.
    fn poll_recv(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_recv(cx)
            .map_err(|e| EvalError::Communication(e.to_string()))
    }
}
