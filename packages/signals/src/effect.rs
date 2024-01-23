use crate::write::*;
use core::{self, fmt::Debug};
use dioxus_core::prelude::*;
use futures_channel::mpsc::UnboundedSender;
use futures_util::StreamExt;
use generational_box::GenerationalBoxId;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::fmt::{self, Formatter};

use crate::CopyValue;

thread_local! {
    pub(crate)static EFFECT_STACK: EffectStack = EffectStack::default();
}

pub(crate) struct EffectStack {
    pub(crate) effects: RwLock<Vec<Effect>>,
    pub(crate) effect_mapping: RwLock<FxHashMap<GenerationalBoxId, Effect>>,
}

impl Default for EffectStack {
    fn default() -> Self {
        Self {
            effects: RwLock::new(Vec::new()),
            effect_mapping: RwLock::new(FxHashMap::default()),
        }
    }
}

impl EffectStack {
    pub(crate) fn current(&self) -> Option<Effect> {
        self.effects.read().last().copied()
    }
}

/// This is a thread safe reference to an effect stack running on another thread.
#[derive(Clone)]
pub(crate) struct EffectStackRef {
    rerun_effect: UnboundedSender<GenerationalBoxId>,
}

impl EffectStackRef {
    pub(crate) fn rerun_effect(&self, id: GenerationalBoxId) {
        self.rerun_effect.unbounded_send(id).unwrap();
    }
}

pub(crate) fn get_effect_ref() -> EffectStackRef {
    match try_consume_context() {
        Some(rt) => rt,
        None => {
            let (sender, mut receiver) = futures_channel::mpsc::unbounded();
            spawn_forever(async move {
                while let Some(id) = receiver.next().await {
                    EFFECT_STACK.with(|stack| {
                        let effect_mapping = stack.effect_mapping.read();
                        if let Some(mut effect) = effect_mapping.get(&id).copied() {
                            tracing::trace!("Rerunning effect: {:?}", id);
                            effect.try_run();
                        } else {
                            tracing::trace!("Effect not found: {:?}", id);
                        }
                    });
                }
            });
            let stack_ref = EffectStackRef {
                rerun_effect: sender,
            };
            provide_root_context(stack_ref.clone());
            stack_ref
        }
    }
}

/// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
/// The signal will be owned by the current component and will be dropped when the component is dropped.
pub fn use_effect(callback: impl FnMut() + 'static) {
    use_hook(|| Effect::new(callback));
}

/// Effects allow you to run code when a signal changes. Effects are run immediately and whenever any signal it reads changes.
#[derive(Copy, Clone, PartialEq)]
pub struct Effect {
    pub(crate) source: ScopeId,
    pub(crate) inner: CopyValue<EffectInner>,
}

pub(crate) struct EffectInner {
    pub(crate) callback: Box<dyn FnMut()>,
    pub(crate) id: GenerationalBoxId,
}

impl EffectInner {
    pub(crate) fn new(callback: Box<dyn FnMut()>) -> CopyValue<Self> {
        let mut copy = CopyValue::invalid();
        let inner = EffectInner {
            callback: Box::new(callback),
            id: copy.id(),
        };
        copy.set(inner);
        copy
    }
}

impl Drop for EffectInner {
    fn drop(&mut self) {
        EFFECT_STACK.with(|stack| {
            tracing::trace!("Dropping effect: {:?}", self.id);
            stack.effect_mapping.write().remove(&self.id);
        });
    }
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:?}", self.inner.value))
    }
}

impl Effect {
    pub(crate) fn current() -> Option<Self> {
        EFFECT_STACK.with(|stack| stack.effects.read().last().copied())
    }

    /// Create a new effect. The effect will be run immediately and whenever any signal it reads changes.
    ///
    /// The signal will be owned by the current component and will be dropped when the component is dropped.
    pub fn new(callback: impl FnMut() + 'static) -> Self {
        let mut myself = Self {
            source: current_scope_id().expect("in a virtual dom"),
            inner: EffectInner::new(Box::new(callback)),
        };

        EFFECT_STACK.with(|stack| {
            stack
                .effect_mapping
                .write()
                .insert(myself.inner.id(), myself);
        });
        tracing::trace!("Created effect: {:?}", myself);

        myself.try_run();

        myself
    }

    /// Run the effect callback immediately. Returns `true` if the effect was run. Returns `false` is the effect is dead.
    pub fn try_run(&mut self) {
        tracing::trace!("Running effect: {:?}", self);
        if let Ok(mut inner) = self.inner.try_write() {
            {
                EFFECT_STACK.with(|stack| {
                    stack.effects.write().push(*self);
                });
            }
            (inner.callback)();
            {
                EFFECT_STACK.with(|stack| {
                    stack.effects.write().pop();
                });
            }
        }
    }

    /// Get the id of this effect.
    pub fn id(&self) -> GenerationalBoxId {
        self.inner.id()
    }
}
