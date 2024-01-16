use dioxus_core::prelude::*;

use crate::dependency::Dependency;
use crate::use_signal;
use crate::{get_effect_stack, signal::SignalData, CopyValue, Effect, ReadOnlySignal, Signal};

/// Creates a new Selector. The selector will be run immediately and whenever any signal it reads changes.
///
/// Selectors can be used to efficiently compute derived data from signals.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App() -> Element {
///     let mut count = use_signal(|| 0);
///     let double = use_selector(move || count * 2);
///     count += 1;
///     assert_eq!(double.value(), count * 2);
///
///     rsx! { "{double}" }
/// }
/// ```
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_selector<R: PartialEq>(f: impl FnMut() -> R + 'static) -> ReadOnlySignal<R> {
    use_hook(|| selector(f))
}

/// Creates a new Selector. The selector will be run immediately and whenever any signal it reads changes.
///
/// Selectors can be used to efficiently compute derived data from signals.
pub fn selector<R: PartialEq>(mut f: impl FnMut() -> R + 'static) -> ReadOnlySignal<R> {
    let mut state = Signal::<R> {
        inner: CopyValue::invalid(),
    };
    let effect = Effect {
        source: current_scope_id().expect("in a virtual dom"),
        callback: CopyValue::invalid(),
        effect_stack: get_effect_stack(),
    };

    {
        get_effect_stack().effects.write().push(effect);
    }
    state.inner.value.set(SignalData {
        subscribers: Default::default(),
        effect_subscribers: Default::default(),
        value: f(),
        effect_stack: get_effect_stack(),
        update_any: schedule_update_any(),
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
