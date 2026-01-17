use dioxus_core::{use_hook, Runtime};

/// Run a closure after the suspense boundary this is under is resolved. The
/// closure will be run immediately if the suspense boundary is already resolved
/// or the scope is not under a suspense boundary.
pub fn use_after_suspense_resolved(suspense_resolved: impl FnOnce() + 'static) {
    use_hook(|| {
        // If this is under a suspense boundary, we need to check if it is resolved
        match Runtime::current().suspense_context() {
            Some(context) => {
                // If it is suspended, run the closure after the suspense is resolved
                context.after_suspense_resolved(suspense_resolved)
            }
            None => {
                // Otherwise, just run the resolved closure immediately
                suspense_resolved();
            }
        }
    })
}
