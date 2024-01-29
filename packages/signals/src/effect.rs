use crate::write::*;
use core::{self, fmt::Debug};
use dioxus_core::prelude::*;
use futures_channel::mpsc::UnboundedSender;
use futures_util::{future::Either, pin_mut, StreamExt};
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
    if let Some(rt) = try_consume_context() {
        return rt;
    }

    let (sender, receiver) = futures_channel::mpsc::unbounded();

    spawn_forever(async move { effect_driver(receiver).await });

    let stack_ref = EffectStackRef {
        rerun_effect: sender,
    };

    provide_root_context(stack_ref.clone());

    stack_ref
}

/// The primary top-level driver of all effects
///
/// In Dioxus, effects are neither react effects nor solidjs effects. They are a hybrid of the two, making our model
/// more complex but also more powerful.
///
/// In react, when a component renders, it can queue up effects to be run after the component is done rendering.
/// This is done *only during render* and determined by the dependency array attached to the effect. In Dioxus,
/// we track effects using signals, so these effects can actually run multiple times after the component has rendered.
///
///
async fn effect_driver(
    mut receiver: futures_channel::mpsc::UnboundedReceiver<GenerationalBoxId>,
) -> ! {
    let mut queued_memos = Vec::new();

    loop {
        // Wait for a flush
        // This gives a chance for effects to be updated in place and memos to compute their values
        let flush_await = flush_sync();
        pin_mut!(flush_await);

        // Until the flush is ready, wait for a new effect to be queued
        // We don't run the effects immediately because we want to batch them on the next call to flush
        // todo: the queued memos should be unqueued when components are dropped
        loop {
            match futures_util::future::select(&mut flush_await, receiver.next()).await {
                // VDOM is flushed and we can run the queued effects
                Either::Left(_flushed) => break,

                // A new effect was queued to be run after the next flush
                // Marking components as dirty is handled syncrhonously on write, though we could try
                // batching them here too
                Either::Right((_queued, _)) => {
                    if let Some(task) = _queued {
                        queued_memos.push(task);
                    }
                }
            }
        }

        EFFECT_STACK.with(|stack| {
            for id in queued_memos.drain(..) {
                let effect_mapping = stack.effect_mapping.read();
                if let Some(mut effect) = effect_mapping.get(&id).copied() {
                    tracing::trace!("Rerunning effect: {:?}", id);
                    effect.try_run();
                } else {
                    tracing::trace!("Effect not found: {:?}", id);
                }
            }
        });
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
    pub fn new(mut callback: impl FnMut() + 'static) -> Self {
        let source = current_scope_id().expect("in a virtual dom");
        let myself = Self {
            source,
            #[allow(clippy::redundant_closure)]
            inner: EffectInner::new(Box::new(move || source.in_runtime(|| callback()))),
        };

        EFFECT_STACK.with(|stack| {
            stack
                .effect_mapping
                .write()
                .insert(myself.inner.id(), myself);
        });
        tracing::trace!("Created effect: {:?}", myself);

        get_effect_ref().rerun_effect(myself.inner.id());

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
