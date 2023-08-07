use dioxus_core::prelude::*;

use crate::{get_effect_stack, CopyValue, Effect, Signal, SignalData};

pub fn use_memo<R: PartialEq>(cx: &ScopeState, f: impl FnMut() -> R + 'static) -> Signal<R> {
    *cx.use_hook(|| memo(f))
}

pub fn memo<R: PartialEq>(mut f: impl FnMut() -> R + 'static) -> Signal<R> {
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
            let state = state.read();
            value != *state
        };
        if changed {
            state.set(value)
        }
    }));

    state
}
