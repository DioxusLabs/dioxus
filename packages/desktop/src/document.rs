use crate::{DesktopContext, WeakDesktopContext, edits::WryQueue, query::Query};
use dioxus_document::{
    Document, Eval, EvalError, Evaluator, GlobalListenerOptions, GlobalListenerRegistration,
    GlobalListeners, LinkProps, MetaProps, ScriptProps, StyleProps, create_element_in_head,
};
use dioxus_html::{EventTargetKind, GlobalEventCallback, GlobalEventInput};
use dioxus_interpreter_js::GlobalListenerFlags;
use dioxus_vdom::queue_effect;
use std::{cell::Cell, cell::RefCell, collections::BTreeMap, rc::Rc};

use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};

/// Code for the Dioxus channel used to communicate between the dioxus and javascript code
pub const NATIVE_EVAL_JS: &str = include_str!("./js/native_eval.js");

/// The window and document listeners registered in one webview.
pub(crate) struct DesktopGlobalListeners {
    next_id: Cell<u32>,
    registrations: RefCell<BTreeMap<u32, (GlobalEventCallback, &'static str, EventTargetKind)>>,
    edit_queue: WryQueue,
}

impl DesktopGlobalListeners {
    pub(crate) fn new(edit_queue: WryQueue) -> Self {
        Self {
            next_id: Cell::new(0),
            registrations: RefCell::default(),
            edit_queue,
        }
    }

    /// The callback, event name and target of the listener `id`.
    pub(crate) fn get(
        &self,
        id: u32,
    ) -> Option<(GlobalEventCallback, &'static str, EventTargetKind)> {
        self.registrations.borrow().get(&id).cloned()
    }
}

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

    /// Register the listener through the webview's edit stream, so it is attached in order with
    /// the DOM edits of the render that produced it.
    fn add_global_listener(
        &self,
        target: EventTargetKind,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration> {
        let ctx = self.desktop_ctx.upgrade()?;
        let passive = options.passive;
        let callback: GlobalEventCallback = Rc::new(move |mut input: GlobalEventInput| {
            input.cancelable &= !passive;
            callback(input)
        });
        let listeners = &ctx.global_listeners;
        let id = listeners.next_id.get();
        listeners.next_id.set(id.checked_add(1)?);
        listeners
            .registrations
            .borrow_mut()
            .insert(id, (callback, name, target));
        let flags = GlobalListenerFlags {
            window: target == EventTargetKind::Window,
            capture: options.capture,
            passive,
        };
        listeners
            .edit_queue
            .with_mutation_state(|state| state.add_global_event_listener(id, name, flags));
        // Listeners are registered from effects, after the render's edits were written. Send
        // them now rather than with the next render so the listener is attached promptly.
        listeners.edit_queue.send_edits();
        let ctx = self.desktop_ctx.clone();
        Some(GlobalListenerRegistration::new(move || {
            if let Some(ctx) = ctx.upgrade() {
                let listeners = &ctx.global_listeners;
                listeners.registrations.borrow_mut().remove(&id);
                listeners
                    .edit_queue
                    .with_mutation_state(|state| state.remove_global_event_listener(id));
            }
        }))
    }
}

impl GlobalListeners for DesktopDocument {
    fn add_window_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration> {
        self.add_global_listener(EventTargetKind::Window, name, options, callback)
    }

    fn add_document_listener(
        &self,
        name: &'static str,
        options: &GlobalListenerOptions,
        callback: GlobalEventCallback,
    ) -> Option<GlobalListenerRegistration> {
        self.add_global_listener(EventTargetKind::Document, name, options, callback)
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
