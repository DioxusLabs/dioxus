use dioxus_core::prelude::*;

use crate::{get_effect_stack, signal::SignalData, CopyValue, Effect, ReadOnlySignal, Signal};

/// Creates a new Selector. The selector will be run immediately and whenever any signal it reads changes.
///
/// Selectors can be used to efficiently compute derived data from signals.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App(cx: Scope) -> Element {
///     let mut count = use_signal(cx, || 0);
///     let double = use_selector(cx, move || count * 2);
///     count += 1;
///     assert_eq!(double.value(), count * 2);
///  
///     render! { "{double}" }
/// }
/// ```
pub fn use_selector<R: PartialEq>(
    cx: &ScopeState,
    f: impl FnMut() -> R + 'static,
) -> ReadOnlySignal<R> {
    *cx.use_hook(|| selector(f))
}

/// Creates a new Selector. The selector will be run immediately and whenever any signal it reads changes.
///
/// Selectors can be used to efficiently compute derived data from signals.
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
