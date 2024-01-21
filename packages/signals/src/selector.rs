use dioxus_core::prelude::*;
use generational_box::Storage;

use crate::dependency::Dependency;
use crate::use_signal;
use crate::{signal::SignalData, ReadOnlySignal, Signal};

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
///     let double = use_memo(move || count * 2);
///     count += 1;
///     assert_eq!(double.value(), count * 2);
///
///     rsx! { "{double}" }
/// }
/// ```
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_memo<R: PartialEq>(f: impl FnMut() -> R + 'static) -> ReadOnlySignal<R> {
    use_maybe_sync_memo(f)
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
///     let double = use_memo(cx, move || count * 2);
///     count += 1;
///     assert_eq!(double.value(), count * 2);
///
///     render! { "{double}" }
/// }
/// ```
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_maybe_sync_memo<R: PartialEq, S: Storage<SignalData<R>>>(
    f: impl FnMut() -> R + 'static,
) -> ReadOnlySignal<R, S> {
    use_hook(|| Signal::maybe_sync_memo(f))
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
///     let double = use_memo_with_dependencies(cx, (local_state.get(),), move |(local_state,)| local_state * 2);
///     local_state.set(1);
///
///     render! { "{double}" }
/// }
/// ```
#[track_caller]
#[must_use = "Consider using `use_effect` to rerun a callback when dependencies change"]
pub fn use_memo_with_dependencies<R: PartialEq, D: Dependency>(
    dependencies: D,
    f: impl FnMut(D::Out) -> R + 'static,
) -> ReadOnlySignal<R>
where
    D::Out: 'static,
{
    use_maybe_sync_selector_with_dependencies(dependencies, f)
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
///     let double = use_memo_with_dependencies(cx, (local_state.get(),), move |(local_state,)| local_state * 2);
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
    dependencies: D,
    mut f: impl FnMut(D::Out) -> R + 'static,
) -> ReadOnlySignal<R, S>
where
    D::Out: 'static,
{
    let mut dependencies_signal = use_signal(|| dependencies.out());
    let selector = use_hook(|| {
        Signal::maybe_sync_memo(move || {
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
