use dioxus_core::prelude::*;

use crate::CopyValue;

#[derive(Default, Clone, Copy)]
pub(crate) struct EffectStack {
    pub(crate) effects: CopyValue<Vec<Effect>>,
}

pub(crate) fn get_effect_stack() -> EffectStack {
    match consume_context() {
        Some(rt) => rt,
        None => {
            let store = EffectStack::default();
            provide_root_context(store).expect("in a virtual dom")
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Effect {
    callback: CopyValue<Box<dyn FnMut()>>,
}

impl Effect {
    pub(crate) fn current() -> Option<Self> {
        get_effect_stack().effects.read().last().copied()
    }

    pub fn new(callback: impl FnMut() + 'static) -> Self {
        let myself = Self {
            callback: CopyValue::new(Box::new(callback)),
        };

        myself.try_run();

        myself
    }

    /// Run the effect callback immediately. Returns `true` if the effect was run. Returns `false` is the effect is dead.
    pub fn try_run(&self) {
        if let Some(mut callback) = self.callback.try_write() {
            {
                get_effect_stack().effects.write().push(*self);
            }
            callback();
            {
                get_effect_stack().effects.write().pop();
            }
        }
    }
}
