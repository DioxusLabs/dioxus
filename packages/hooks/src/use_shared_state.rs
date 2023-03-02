use dioxus_core::{ScopeId, ScopeState};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashSet,
    rc::Rc,
    sync::Arc,
};

type ProvidedState<T> = Rc<RefCell<ProvidedStateInner<T>>>;

// Tracks all the subscribers to a shared State
pub(crate) struct ProvidedStateInner<T> {
    value: T,
    notify_any: Arc<dyn Fn(ScopeId)>,
    consumers: HashSet<ScopeId>,
}

impl<T> ProvidedStateInner<T> {
    pub(crate) fn notify_consumers(&mut self) {
        for consumer in self.consumers.iter() {
            (self.notify_any)(*consumer);
        }
    }
}

/// This hook provides some relatively light ergonomics around shared state.
///
/// It is not a substitute for a proper state management system, but it is capable enough to provide use_state - type
/// ergonomics in a pinch, with zero cost.
///
/// # Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// #
/// # fn app(cx: Scope) -> Element {
/// #     render! {
/// #         Parent{}
/// #     }
/// # }
///
/// #[derive(Clone, Copy)]
/// enum Theme {
///     Light,
///     Dark,
/// }
///
/// // Provider
/// fn Parent<'a>(cx: Scope<'a>) -> Element<'a> {
///     use_shared_state_provider(cx, || Theme::Dark);
///     let theme = use_shared_state::<Theme>(cx).unwrap();
///
///     render! {
///         button{
///             onclick: move |_| {
///                 let current_theme = *theme.read();
///                 *theme.write() = match current_theme {
///                     Theme::Dark => Theme::Light,
///                     Theme::Light => Theme::Dark,
///                 };
///             },
///             "Change theme"
///         }
///         Child{}
///     }
/// }
///
/// // Consumer
/// fn Child<'a>(cx: Scope<'a>) -> Element<'a> {
///     let theme = use_shared_state::<Theme>(cx).unwrap();
///     let current_theme = *theme.read();
///
///     render! {
///         match &*theme.read() {
///             Theme::Dark => {
///                 "Dark mode"
///             }
///             Theme::Light => {
///                 "Light mode"
///             }
///         }
///     }
/// }
/// ```
///
/// # How it works
///
/// Any time a component calls `write`, every consumer of the state will be notified - excluding the provider.
///
/// Right now, there is not a distinction between read-only and write-only, so every consumer will be notified.
pub fn use_shared_state<T: 'static>(cx: &ScopeState) -> Option<&UseSharedState<T>> {
    let state: &Option<UseSharedStateOwner<T>> = &*cx.use_hook(move || {
        let scope_id = cx.scope_id();
        let root = cx.consume_context::<ProvidedState<T>>()?;

        root.borrow_mut().consumers.insert(scope_id);

        let state = UseSharedState { inner: root };
        let owner = UseSharedStateOwner { state, scope_id };
        Some(owner)
    });
    state.as_ref().map(|s| &s.state)
}

/// This wrapper detects when the hook is dropped and will unsubscribe when the component is unmounted
struct UseSharedStateOwner<T> {
    state: UseSharedState<T>,
    scope_id: ScopeId,
}

impl<T> Drop for UseSharedStateOwner<T> {
    fn drop(&mut self) {
        // we need to unsubscribe when our component is unmounted
        let mut root = self.state.inner.borrow_mut();
        root.consumers.remove(&self.scope_id);
    }
}

/// State that is shared between components through the context system
pub struct UseSharedState<T> {
    pub(crate) inner: Rc<RefCell<ProvidedStateInner<T>>>,
}

impl<T> UseSharedState<T> {
    /// Notify all consumers of the state that it has changed. (This is called automatically when you call "write")
    pub fn notify_consumers(&self) {
        self.inner.borrow_mut().notify_consumers();
    }

    /// Read the shared value
    pub fn read(&self) -> Ref<'_, T> {
        Ref::map(self.inner.borrow(), |inner| &inner.value)
    }

    /// Calling "write" will force the component to re-render
    ///
    ///
    // TODO: We prevent unncessary notifications only in the hook, but we should figure out some more global lock
    pub fn write(&self) -> RefMut<'_, T> {
        let mut value = self.inner.borrow_mut();
        value.notify_consumers();
        RefMut::map(value, |inner| &mut inner.value)
    }

    /// Allows the ability to write the value without forcing a re-render
    pub fn write_silent(&self) -> RefMut<'_, T> {
        RefMut::map(self.inner.borrow_mut(), |inner| &mut inner.value)
    }
}

impl<T> Clone for UseSharedState<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for UseSharedState<T> {
    fn eq(&self, other: &Self) -> bool {
        let first = self.inner.borrow();
        let second = other.inner.borrow();
        first.value == second.value
    }
}

/// Provide some state for components down the hierarchy to consume without having to drill props. See [`use_shared_state`] to consume the state
///
///
/// # Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// #
/// # fn app(cx: Scope) -> Element {
/// #     render! {
/// #         Parent{}
/// #     }
/// # }
///
/// #[derive(Clone, Copy)]
/// enum Theme {
///     Light,
///     Dark,
/// }
///
/// // Provider
/// fn Parent<'a>(cx: Scope<'a>) -> Element<'a> {
///     use_shared_state_provider(cx, || Theme::Dark);
///     let theme = use_shared_state::<Theme>(cx).unwrap();
///
///     render! {
///         button{
///             onclick: move |_| {
///                 let current_theme = *theme.read();
///                 *theme.write() = match current_theme {
///                     Theme::Dark => Theme::Light,
///                     Theme::Light => Theme::Dark,
///                 };
///             },
///             "Change theme"
///         }
///         // Children components that consume the state...
///     }
/// }
/// ```
pub fn use_shared_state_provider<T: 'static>(cx: &ScopeState, f: impl FnOnce() -> T) {
    cx.use_hook(|| {
        let state: ProvidedState<T> = Rc::new(RefCell::new(ProvidedStateInner {
            value: f(),
            notify_any: cx.schedule_update_any(),
            consumers: HashSet::new(),
        }));

        cx.provide_context(state);
    });
}
