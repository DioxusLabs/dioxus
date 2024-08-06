use crate::use_callback;
use dioxus_core::prelude::*;
use dioxus_signals::{Memo, Signal};

#[doc = include_str!("../docs/derived_state.md")]
#[doc = include_str!("../docs/rules_of_hooks.md")]
#[doc = include_str!("../docs/moving_state_around.md")]
#[track_caller]
pub fn use_memo<R: PartialEq>(mut f: impl FnMut() -> R + 'static) -> Memo<R> {
    let callback = use_callback(move |_| f());
    #[allow(clippy::redundant_closure)]
    use_hook(|| Signal::memo(move || callback(())))
}
