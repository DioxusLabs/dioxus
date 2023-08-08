use core::{self, fmt::Debug};
use std::fmt::{self, Formatter};
use std::marker::PhantomData;

use dioxus_core::prelude::*;

use crate::dependency::Dep;
use crate::{dependency::Dependency, CopyValue};
use crate::{use_signal, Signal};

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

/// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
/// The signal will be owned by the current component and will be dropped when the component is dropped.
pub fn use_effect(cx: &ScopeState, callback: impl FnMut() + 'static) {
    cx.use_hook(|| Effect::new(callback));
}

/// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
/// The signal will be owned by the current component and will be dropped when the component is dropped.
pub fn use_effect_with_dependencies<D: Dependency>(
    cx: &ScopeState,
    dependencies: D,
    mut callback: impl FnMut(D::Out) + 'static,
) where
    D::Out: 'static,
{
    let dependencies_signal = use_signal(cx, || dependencies.out());
    cx.use_hook(|| {
        Effect::new(move || {
            let deref = &*dependencies_signal.read();
            callback(deref.clone());
        });
    });
    let changed = { dependencies.changed(&*dependencies_signal.read()) };
    if changed {
        dependencies_signal.set(dependencies.out());
    }
}

/// Effects allow you to run code when a signal changes. Effects are run immediately and whenever any signal it reads changes.
#[derive(Copy, Clone, PartialEq)]
pub struct Effect {
    pub(crate) callback: CopyValue<Box<dyn FnMut()>>,
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:?}", self.callback.value))
    }
}

impl Effect {
    pub(crate) fn current() -> Option<Self> {
        get_effect_stack().effects.read().last().copied()
    }

    /// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
    ///
    /// The signal will be owned by the current component and will be dropped when the component is dropped.
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
