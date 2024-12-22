use dioxus_core::prelude::queue_effect;
use dioxus_core::ScopeId;
use dioxus_document::{
    create_element_in_head, Document, Eval, EvalError, Evaluator, LinkProps, MetaProps,
    ScriptProps, StyleProps,
};
use dioxus_history::History;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage};
use std::rc::Rc;

use crate::history::LiveviewHistory;
use crate::query::{Query, QueryEngine};

/// Represents a liveview-target's JavaScript evaluator.
pub(crate) struct LiveviewEvaluator {
    query: Query<serde_json::Value>,
}

impl LiveviewEvaluator {
    /// Creates a new evaluator for liveview-based targets.
    pub fn create(query_engine: QueryEngine, js: String) -> GenerationalBox<Box<dyn Evaluator>> {
        let query = query_engine.new_query(&js);
        // We create a generational box that is owned by the query slot so that when we drop the query slot, the generational box is also dropped.
        let owner = UnsyncStorage::owner();
        let query_id = query.id;
        let query = owner.insert(Box::new(LiveviewEvaluator { query }) as Box<dyn Evaluator>);
        query_engine.active_requests.slab.borrow_mut()[query_id].owner = Some(owner);

        query
    }
}

impl Evaluator for LiveviewEvaluator {
    /// # Panics
    /// This will panic if the query is currently being awaited.
    fn poll_join(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_result(context)
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
    ///
    /// # Panics
    /// This will panic if the query is currently being awaited.
    fn poll_recv(
        &mut self,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<serde_json::Value, EvalError>> {
        self.query
            .poll_recv(context)
            .map_err(|e| EvalError::Communication(e.to_string()))
    }
}

/// Provides the LiveviewDocument through [`ScopeId::provide_context`].
pub fn init_document() {
    let query = ScopeId::ROOT.consume_context::<QueryEngine>().unwrap();
    let provider: Rc<dyn Document> = Rc::new(LiveviewDocument {
        query: query.clone(),
    });
    ScopeId::ROOT.provide_context(provider);
    let history = LiveviewHistory::new(Rc::new(move |script: &str| {
        Eval::new(LiveviewEvaluator::create(query.clone(), script.to_string()))
    }));
    let history: Rc<dyn History> = Rc::new(history);
    ScopeId::ROOT.provide_context(history);
}

/// Reprints the liveview-target's provider of evaluators.
#[derive(Clone)]
pub struct LiveviewDocument {
    query: QueryEngine,
}

impl Document for LiveviewDocument {
    fn eval(&self, js: String) -> Eval {
        Eval::new(LiveviewEvaluator::create(self.query.clone(), js))
    }

    /// Set the title of the document
    fn set_title(&self, title: String) {
        let myself = self.clone();
        queue_effect(move || {
            myself.eval(format!("document.title = {title:?};"));
        });
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
