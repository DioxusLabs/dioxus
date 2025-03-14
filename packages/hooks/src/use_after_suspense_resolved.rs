use dioxus_core::{
    generation,
    prelude::{current_scope_id, needs_update},
    use_hook,
};

/// Run a closure after the suspense boundary this hook is called under is resolved
pub fn use_after_suspense_resolved(suspense_resolved: impl FnOnce()) {
    let closure_ran = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));
    if closure_ran.get() {
        return;
    }
    let run_closure = || {
        closure_ran.set(true);
        suspense_resolved();
    };
    let scope = current_scope_id().unwrap();
    // If this is under a suspense boundary, we need to check if it is resolved
    if scope.under_suspense_boundary() {
        // If it is suspended, it will rerun this component when it is resolved
        if !scope.is_suspended() {
            // Otherwise if this is the first run, we need to wait until either
            // the suspense boundary is resolved or not
            if generation() == 0 {
                needs_update();
            } else {
                // If this isn't the first run and the suspense boundary is resolved,
                // run the resolved closure
                run_closure();
            }
        }
    } else {
        // Otherwise, just run the resolved closure immediately
        run_closure();
    }
}
