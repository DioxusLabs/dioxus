//! Eager reactive computations — the "components" of the framework.

use std::marker::PhantomData;

use crate::runtime::{NodeId, UpdateResult, with_rt};

/// An eager computation that reruns when values it read change.
///
/// Effects are also ownership scopes: stores, memos, effects, and DOM
/// bindings created while an effect runs are owned by it and are dropped —
/// children first — right before it reruns, and when it is dropped. Nesting
/// effects therefore behaves like nesting components: conditional UI built
/// inside an effect tears itself down when the condition changes.
pub struct Effect {
    id: NodeId,
    _marker: PhantomData<*const ()>,
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for Effect {}

impl Effect {
    /// Create the effect and run it once immediately.
    pub fn new(mut run: impl FnMut() + 'static) -> Self {
        let id = with_rt(|rt| {
            let id = rt.create_node(None, None, true);
            rt.set_update(
                id,
                Box::new(move || {
                    run();
                    UpdateResult::Unchanged
                }),
            );
            rt.ensure(id);
            id
        });
        Effect {
            id,
            _marker: PhantomData,
        }
    }

    /// Drop the effect and everything it owns. Effects owned by an enclosing
    /// effect are dropped automatically; only call this for top-level ones.
    pub fn dispose(self) {
        with_rt(|rt| rt.drop_node(self.id));
    }

    pub fn is_alive(&self) -> bool {
        with_rt(|rt| rt.is_alive(self.id))
    }
}

/// Convenience: `effect(f)` is `Effect::new(f)`.
pub fn effect(run: impl FnMut() + 'static) -> Effect {
    Effect::new(run)
}
