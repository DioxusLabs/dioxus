use dioxus_core::prelude::*;
use dioxus_signals::ReactiveContext;

/// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
/// The signal will be owned by the current component and will be dropped when the component is dropped.
///
/// If the use_effect call was skipped due to an early return, the effect will no longer activate.
pub fn use_effect(mut callback: impl FnMut() + 'static) {
    // let mut run_effect = use_hook(|| CopyValue::new(true));
    // use_hook_did_run(move |did_run| run_effect.set(did_run));

    use_hook(|| {
        spawn(async move {
            let rc = ReactiveContext::new();

            loop {
                // Wait for the dom the be finished with sync work
                flush_sync().await;

                // Run the effect
                rc.run_in(&mut callback);

                // Wait for context to change
                rc.changed().await;
            }
        });
    });
}
