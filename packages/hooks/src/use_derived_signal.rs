use crate::use_callback;
use dioxus_core::use_hook;
use dioxus_signals::Signal;

#[doc = include_str!("../docs/derived_signal.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[track_caller]
pub fn use_derived_signal<R: 'static>(mut f: impl FnMut() -> R + 'static) -> Signal<R> {
    let callback = use_callback(move |_| f());
    let caller = std::panic::Location::caller();
    #[allow(clippy::redundant_closure)]
    use_hook(|| Signal::derived_signal_with_location(move || callback(()), caller))
}
