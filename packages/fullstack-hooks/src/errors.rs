//! Contains a hydration compatible error boundary context.

use dioxus_core::{spawn_isomorphic, CapturedError, ErrorContext, ReactiveContext};
use dioxus_fullstack_protocol::serialize_context;
use futures_util::StreamExt;

/// Initializes an error boundary context that is compatible with hydration.
pub fn init_error_boundary() -> ErrorContext {
    let initial_errors = serialize_context().create_entry::<Vec<CapturedError>>();
    let (rx_context, mut rx) = ReactiveContext::new();
    let errors = ErrorContext::new(Vec::new());
    for error in initial_errors.get().iter().flatten() {
        errors.insert_error(error.clone());
    }
    rx_context.run_in(|| {
        errors.errors();
    });
    spawn_isomorphic({
        let errors = errors.clone();
        async move {
            if rx.next().await.is_some() {
                rx_context.run_in(|| {
                    initial_errors.insert(&errors.errors().to_vec(), std::panic::Location::caller())
                });
            }
        }
    });
    errors
}
