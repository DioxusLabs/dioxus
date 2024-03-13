use crate::use_callback;
use dioxus_core::prelude::*;
use dioxus_signals::{Memo, Signal};

/// Creates a new  Memo. The memo will be run immediately and whenever any signal it reads changes.
///
/// Memos can be used to efficiently compute derived data from signals.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_signals::*;
///
/// fn App() -> Element {
///     let mut count = use_signal(|| 0);
///     let double = use_memo(move || count * 2);
///     count += 1;
///     assert_eq!(double(), count * 2);
///
///     rsx! { "{double}" }
/// }
/// ```
#[track_caller]
pub fn use_memo<R: PartialEq>(f: impl FnMut() -> R + 'static) -> Memo<R> {
    let callback = use_callback(f);
    #[allow(clippy::redundant_closure)]
    use_hook(|| Signal::memo(move || callback()))
}
