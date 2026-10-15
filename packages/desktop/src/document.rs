use crate::{query::Query, DesktopContext, WeakDesktopContext};
use dioxus_core::queue_effect;
use dioxus_document::{
    create_element_in_head, Document, Eval, EvalError, Evaluator, LinkProps, MetaProps,
    ScriptProps, StyleProps,
};

use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use std::cell::Cell;
use std::rc::Rc;

/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const NATIVE_EVAL_JS: &str = include_str!("./js/native_eval.js");

/// JavaScript that defines the head insertion gate. Every queued head insertion gets a unique id.
/// An eval waits for every insertion id that was queued before it, so an eval never runs before
/// those head elements, no matter in which order the webview executes scripts.
const HEAD_GATE_JS: &str = r#"
window.__dioxusHeadCompleted = window.__dioxusHeadCompleted || new Set();
window.__dioxusHeadSatisfied = window.__dioxusHeadSatisfied || function (target) {
    for (let id = 1; id <= target; id++) {
        if (!window.__dioxusHeadCompleted.has(id)) {
            return false;
        }
    }
    return true;
};
window.__dioxusHeadWait = window.__dioxusHeadWait || function (target) {
    if (window.__dioxusHeadSatisfied(target)) {
        return Promise.resolve();
    }
    return new Promise(function (resolve) {
        (window.__dioxusHeadWaiters = window.__dioxusHeadWaiters || []).push({
            target: target,
            resolve: resolve,
        });
    });
};
window.__dioxusHeadComplete = window.__dioxusHeadComplete || function (id) {
    window.__dioxusHeadCompleted.add(id);
    const waiters = window.__dioxusHeadWaiters || [];
    window.__dioxusHeadWaiters = [];
    for (const waiter of waiters) {
        if (window.__dioxusHeadSatisfied(waiter.target)) {
            waiter.resolve();
        } else {
            window.__dioxusHeadWaiters.push(waiter);
        }
    }
};
"#;

/// JavaScript that runs before every user eval. It waits until every head insertion that was queued
/// before the eval has completed, and then waits for inserted scripts and stylesheets to load.
fn head_ready_js(target: usize) -> String {
    format!(
        "{HEAD_GATE_JS}await window.__dioxusHeadWait({target});\nawait (window.__dioxusHeadReady || Promise.resolve());\n"
    )
}

/// Wrap a head insertion so it completes its id when it finishes, even when it throws.
fn head_insertion_js(js: String, id: usize) -> String {
    format!("{HEAD_GATE_JS}try {{\n{js}\n}} finally {{\nwindow.__dioxusHeadComplete({id});\n}}")
}

/// Completes a queued head insertion when the queued effect runs, or when the effect is dropped
/// before it runs.
struct HeadInsertionGuard {
    desktop_ctx: WeakDesktopContext,
    id: usize,
    armed: bool,
}

impl HeadInsertionGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for HeadInsertionGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Some(ctx) = self.desktop_ctx.upgrade() {
            let js = format!("{HEAD_GATE_JS}window.__dioxusHeadComplete({});", self.id);
            _ = ctx.webview.evaluate_script(&js);
        }
    }
}

/// Represents the desktop-target's provider of evaluators.
#[derive(Clone)]
pub struct DesktopDocument {
    pub(crate) desktop_ctx: WeakDesktopContext,
    /// The id of the last head insertion queued. See [`HEAD_GATE_JS`].
    head_insertions: Rc<Cell<usize>>,
}

impl DesktopDocument {
    pub fn new(desktop_ctx: DesktopContext) -> Self {
        let desktop_ctx = Rc::downgrade(&desktop_ctx);
        Self {
            desktop_ctx,
            head_insertions: Rc::new(Cell::new(0)),
        }
    }

    /// Run javascript with a prelude that runs before the script.
    fn eval_with_prelude(&self, js: String, prelude: &str) -> Eval {
        Eval::new(DesktopEvaluator::create(
            self.desktop_ctx
                .upgrade()
                .expect("Window to exist when document is alive"),
            js,
            prelude,
        ))
    }

    /// Queue a head insertion. Evals wait for its id to complete.
    fn queue_head_element(&self, js: String) {
        let id = self.head_insertions.get() + 1;
        self.head_insertions.set(id);
        let myself = self.clone();
        let mut guard = HeadInsertionGuard {
            desktop_ctx: self.desktop_ctx.clone(),
            id,
            armed: true,
        };
        queue_effect(move || {
            myself.eval_with_prelude(head_insertion_js(js, id), "");
            guard.disarm();
        });
    }
}

impl Document for DesktopDocument {
    fn eval(&self, js: String) -> Eval {
        // Head elements are inserted at runtime on desktop, unlike SSR where they are part of the
        // first HTML byte. Wait for the head so evals observe the same settled head as on the web.
        let target = self.head_insertions.get();
        self.eval_with_prelude(js, &head_ready_js(target))
    }

    fn set_title(&self, title: String) {
        if let Some(ctx) = self.desktop_ctx.upgrade() {
            ctx.set_title(&title);
        }
    }

    /// Create a new meta tag in the head
    fn create_meta(&self, props: MetaProps) {
        self.queue_head_element(create_element_in_head("meta", &props.attributes(), None));
    }

    /// Create a new script tag in the head
    fn create_script(&self, props: ScriptProps) {
        self.queue_head_element(create_element_in_head(
            "script",
            &props.attributes(),
            props.script_contents().ok(),
        ));
    }

    /// Create a new style tag in the head
    fn create_style(&self, props: StyleProps) {
        self.queue_head_element(create_element_in_head(
            "style",
            &props.attributes(),
            props.style_contents().ok(),
        ));
    }

    /// Create a new link tag in the head
    fn create_link(&self, props: LinkProps) {
        self.queue_head_element(create_element_in_head("link", &props.attributes(), None));
    }
}

/// Represents a desktop-target's JavaScript evaluator.
pub(crate) struct DesktopEvaluator {
    query: Query<serde_json::Value>,
}

impl DesktopEvaluator {
    /// Creates a new evaluator for desktop-based targets.
    pub fn create(
        desktop_ctx: DesktopContext,
        js: String,
        prelude: &str,
    ) -> GenerationalBox<Box<dyn Evaluator>> {
        let query = desktop_ctx
            .query
            .new_query(&js, prelude, desktop_ctx.clone());

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
