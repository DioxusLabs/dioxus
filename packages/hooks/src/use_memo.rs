use crate::use_callback;
use dioxus_core::use_hook;
use dioxus_signals::Memo;

#[doc = include_str!("../docs/derived_state.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[track_caller]
pub fn use_memo<R: PartialEq + 'static>(mut f: impl FnMut() -> R + 'static) -> Memo<R> {
    let callback = use_callback(move |_| f());
    let caller = std::panic::Location::caller();
    #[allow(clippy::redundant_closure)]
    use_hook(|| Memo::new_with_location(move || callback(()), caller))
}
