use dioxus_core::ScopeState;
use dioxus_hooks::use_context;

use crate::{eval::EvalResult, query::QueryEngine};

#[derive(Clone)]
pub(crate) struct LiveviewContext {
    query: QueryEngine,
}

impl LiveviewContext {
    pub(crate) fn new(query: QueryEngine) -> Self {
        Self { query }
    }

    pub(crate) fn eval(&self, script: &str) -> EvalResult {
        let query = self.query.new_query::<serde_json::Value>(script);
        EvalResult::new(query)
    }
}

pub(crate) fn use_liveview(cx: &ScopeState) -> &LiveviewContext {
    use_context(cx).expect("LiveviewContext not found")
}
