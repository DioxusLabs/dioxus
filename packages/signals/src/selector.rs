use dioxus_core::prelude::*;

use crate::{get_effect_stack, signal::SignalData, CopyValue, Effect, ReadOnlySignal, Signal};

pub fn use_selector<R: PartialEq>(
    cx: &ScopeState,
    f: impl FnMut() -> R + 'static,
) -> ReadOnlySignal<R> {
    *cx.use_hook(|| selector(f))
}

pub fn selector<R: PartialEq>(mut f: impl FnMut() -> R + 'static) -> ReadOnlySignal<R> {
    let state = Signal::<R> {
        inner: CopyValue::invalid(),
    };
    let effect = Effect {
        callback: CopyValue::invalid(),
    };

    {
        get_effect_stack().effects.write().push(effect);
    }
    state.inner.value.set(SignalData {
        subscribers: Default::default(),
        effect_subscribers: Default::default(),
        update_any: schedule_update_any().expect("in a virtual dom"),
        value: f(),
    });
    {
        get_effect_stack().effects.write().pop();
    }

    effect.callback.value.set(Box::new(move || {
        let value = f();
        let changed = {
            let old = state.inner.read();
            value != old.value
        };
        if changed {
            state.set(value)
        }
    }));

    ReadOnlySignal::new(state)
}
