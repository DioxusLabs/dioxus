//! Contains a hydration compatible error boundary context.

use crate::serialize_context;
use dioxus_core::{spawn_isomorphic, CapturedError, ErrorContext, ReactiveContext};
use futures_util::StreamExt;

/// Initializes an error boundary context that is compatible with hydration.
pub fn init_error_boundary() -> ErrorContext {
    let initial_error = serialize_context().create_entry::<Option<CapturedError>>();
    let (rx_context, mut rx) = ReactiveContext::new();
    let errors = ErrorContext::new(None);
    if let Ok(Some(err)) = initial_error.get() {
        errors.insert_error(err);
    }
    rx_context.run_in(|| {
        errors.error();
    });
    spawn_isomorphic({
        let errors = errors.clone();
        async move {
            if rx.next().await.is_some() {
                rx_context.run_in(|| {
                    initial_error.insert(&errors.error(), std::panic::Location::caller())
                });
            }
        }
    });
    errors
}
