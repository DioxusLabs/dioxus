use dioxus_core::prelude::*;
use dioxus_signals::ReactiveContext;
use futures_util::StreamExt;

use crate::use_callback;

#[doc = include_str!("../docs/side_effects.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[track_caller]
pub fn use_effect(callback: impl FnMut() + 'static) -> Effect {
    let callback = use_callback(callback);

    let location = std::panic::Location::caller();

    use_hook(|| {
        // Inside the effect, we track any reads so that we can rerun the effect if a value the effect reads changes
        let (rc, mut changed) = ReactiveContext::new_with_origin(location);
        // Spawn a task that will run the effect when:
        // 1) The component is first run
        // 2) The effect is rerun due to an async read at any time
        // 3) The effect is rerun in the same tick that the component is rerun: we need to wait for the component to rerun before we can run the effect again
        let queue_effect_for_next_render = move || {
            queue_effect(move || rc.run_in(&*callback));
        };

        queue_effect_for_next_render();
        spawn(async move {
            loop {
                // Wait for context to change
                let _ = changed.next().await;

                // Run the effect
                queue_effect_for_next_render();
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
