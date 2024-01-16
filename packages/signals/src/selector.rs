use dioxus_core::prelude::*;
use generational_box::Storage;

use crate::dependency::Dependency;
use crate::{get_effect_ref, signal::SignalData, CopyValue, Effect, ReadOnlySignal, Signal};
use crate::{use_signal, EffectInner, EFFECT_STACK};

/// Creates a new unsync Selector. The selector will be run immediately and whenever any signal it reads changes.
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
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_selector<R: PartialEq>(f: impl FnMut() -> R + 'static) -> ReadOnlySignal<R> {
    use_maybe_sync_selector(f)
}

/// Creates a new Selector that may be sync. The selector will be run immediately and whenever any signal it reads changes.
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
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_maybe_sync_selector<R: PartialEq, S: Storage<SignalData<R>>>(
    f: impl FnMut() -> R + 'static,
) -> ReadOnlySignal<R, S> {
    use_hook(|| maybe_sync_selector(f))
}

/// Creates a new unsync Selector with some local dependencies. The selector will be run immediately and whenever any signal it reads or any dependencies it tracks changes
///
/// Selectors can be used to efficiently compute derived data from signals.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App(cx: Scope) -> Element {
///     let mut local_state = use_state(cx, || 0);
///     let double = use_selector_with_dependencies(cx, (local_state.get(),), move |(local_state,)| local_state * 2);
///     local_state.set(1);
///  
///     render! { "{double}" }
/// }
/// ```
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_selector_with_dependencies<R: PartialEq, D: Dependency>(
    cx: &ScopeState,
    dependencies: D,
    f: impl FnMut(D::Out) -> R + 'static,
) -> ReadOnlySignal<R>
where
    D::Out: 'static,
{
    use_maybe_sync_selector_with_dependencies(cx, dependencies, f)
}

/// Creates a new Selector that may be sync with some local dependencies. The selector will be run immediately and whenever any signal it reads or any dependencies it tracks changes
///
/// Selectors can be used to efficiently compute derived data from signals.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App(cx: Scope) -> Element {
///     let mut local_state = use_state(cx, || 0);
///     let double = use_selector_with_dependencies(cx, (local_state.get(),), move |(local_state,)| local_state * 2);
///     local_state.set(1);
///  
///     render! { "{double}" }
/// }
/// ```
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_maybe_sync_selector_with_dependencies<
    R: PartialEq,
    D: Dependency,
    S: Storage<SignalData<R>>,
>(
    cx: &ScopeState,
    dependencies: D,
    mut f: impl FnMut(D::Out) -> R + 'static,
) -> ReadOnlySignal<R, S>
where
    D::Out: 'static,
{
    let dependencies_signal = use_signal(cx, || dependencies.out());
    let selector = *cx.use_hook(|| {
        maybe_sync_selector(move || {
            let deref = &*dependencies_signal.read();
            f(deref.clone())
        })
    });
    let changed = { dependencies.changed(&*dependencies_signal.read()) };
    if changed {
        dependencies_signal.set(dependencies.out());
    }
    selector
}

/// Creates a new unsync Selector. The selector will be run immediately and whenever any signal it reads changes.
///
/// Selectors can be used to efficiently compute derived data from signals.
#[track_caller]
pub fn selector<R: PartialEq>(f: impl FnMut() -> R + 'static) -> ReadOnlySignal<R> {
    maybe_sync_selector(f)
}

/// Creates a new Selector that may be Sync + Send. The selector will be run immediately and whenever any signal it reads changes.
///
/// Selectors can be used to efficiently compute derived data from signals.
#[track_caller]
pub fn maybe_sync_selector<R: PartialEq, S: Storage<SignalData<R>>>(
    mut f: impl FnMut() -> R + 'static,
) -> ReadOnlySignal<R, S> {
    let state = Signal::<R, S> {
        inner: CopyValue::invalid(),
    };
    let effect = Effect {
        source: current_scope_id().expect("in a virtual dom"),
        inner: CopyValue::invalid(),
    };

    {
        EFFECT_STACK.with(|stack| stack.effects.write().push(effect));
    }
    state.inner.value.set(SignalData {
        subscribers: Default::default(),
        effect_subscribers: Default::default(),
        update_any: schedule_update_any().expect("in a virtual dom"),
        value: f(),
        effect_ref: get_effect_ref(),
    });
    {
        EFFECT_STACK.with(|stack| stack.effects.write().pop());
    }

    let invalid_id = effect.id();
    tracing::trace!("Creating effect: {:?}", invalid_id);
    effect.inner.value.set(EffectInner {
        callback: Box::new(move || {
            let value = f();
            let changed = {
                let old = state.inner.read();
                value != old.value
            };
            if changed {
                state.set(value)
            }
        }),
        id: invalid_id,
    });
    {
        EFFECT_STACK.with(|stack| stack.effect_mapping.write().insert(invalid_id, effect));
    }

    ReadOnlySignal::new_maybe_sync(state)
}
