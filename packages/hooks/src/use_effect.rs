use dioxus_core::prelude::*;
use dioxus_signals::ReactiveContext;
use futures_util::StreamExt;

use crate::use_callback;

#[doc = include_str!("../docs/side_effects.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[track_caller]
pub fn use_effect(callback: impl FnMut() + 'static) -> Effect {
    // let mut run_effect = use_hook(|| CopyValue::new(true));
    // use_hook_did_run(move |did_run| run_effect.set(did_run));

    let callback = use_callback(callback);

    let location = std::panic::Location::caller();

    use_hook(|| {
        let (rc, mut changed) = ReactiveContext::new_with_origin(location);
        spawn(async move {
            loop {
                // Run the effect
                rc.run_in(&*callback);

                // Wait for context to change
                let _ = changed.next().await;

                // Wait for the dom the be finished with sync work
                wait_for_next_render().await;
            }
        });
        Effect { rc }
    })
}

/// A handle to an effect.
#[derive(Clone, Copy)]
pub struct Effect {
    rc: ReactiveContext,
}

impl Effect {
    /// Marks the effect as dirty, causing it to rerun on the next render.
    pub fn mark_dirty(&mut self) {
        self.rc.mark_dirty();
    }
}
