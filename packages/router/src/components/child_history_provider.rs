use dioxus_history::{history, provide_history_context, LensHistory};
use dioxus_lib::prelude::*;

use std::rc::Rc;

use crate::prelude::Routable;

/// Props for the [`ChildHistoryProvider`] component.
#[derive(Props, Clone)]
pub struct ChildHistoryProviderProps<R: Routable> {
    /// The child router to render
    router: R,
    /// Take a parent route and return a child route or none if the route is not part of the child
    parent_to_child_route: fn(&str) -> Option<String>,
    /// Take a child route and return a parent route
    child_to_parent_route: fn(&str) -> String,
}

impl<R: Routable> PartialEq for ChildHistoryProviderProps<R> {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

/// A component that provides a [`History`] to a child router. The `#[child]` attribute on the router macro will insert this automatically.
#[component]
#[allow(missing_docs)]
pub fn ChildHistoryProvider<R: Routable>(props: ChildHistoryProviderProps<R>) -> Element {
    use_hook(|| {
        let parent_history = history();
        let child_history = LensHistory::new(
            parent_history,
            props.parent_to_child_route,
            props.child_to_parent_route,
        );
        provide_history_context(Rc::new(child_history))
    });

    props.router.render(0)
}
