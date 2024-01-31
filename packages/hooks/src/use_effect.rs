use crate::use_hook_did_run;
use dioxus_core::prelude::*;
use dioxus_signals::{CopyValue, Effect, Writable};

/// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
/// The signal will be owned by the current component and will be dropped when the component is dropped.
///
/// If the use_effect call was skipped due to an early return, the effect will no longer activate.
pub fn use_effect(mut callback: impl FnMut() + 'static) {
    let mut run_effect = use_hook(|| CopyValue::new(true));

    use_hook_did_run(move |did_run| match did_run {
        true => run_effect.set(true),
        false => run_effect.set(false),
    });

    use_hook(|| {
        Effect::new(move || {
            if run_effect() {
                callback();
            }
        })
    });
}
