use dioxus_core::{Callback, Element, use_hook};
use dioxus_core_macro::{Props, component};
use dioxus_history::{History, provide_history_context};

use std::rc::Rc;

/// A component that provides a [`History`] for all child [`Router`] components. Renderers generally provide a default history automatically.
#[component]
#[allow(missing_docs)]
pub fn HistoryProvider(
    /// The history to provide to child components.
    history: Callback<(), Rc<dyn History>>,
    /// The children to render within the history provider.
    children: Element,
) -> Element {
    use_hook(|| {
        provide_history_context(history(()));
    });

    children
}
