use self::error::{UseSharedStateError, UseSharedStateResult};
use dioxus_core::ScopeId;
use std::{collections::HashSet, rc::Rc, sync::Arc};

#[cfg(debug_assertions)]
pub use dioxus_debug_cell::{
    error::{BorrowError, BorrowMutError},
    Ref, RefCell, RefMut,
};

#[cfg(not(debug_assertions))]
pub use std::cell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut};

#[macro_export]
macro_rules! debug_location {
    () => {{
        #[cfg(debug_assertions)]
        {
            std::panic::Location::caller()
        }
        #[cfg(not(debug_assertions))]
        {
            ()
        }
    }};
}

pub mod error {
    #[cfg(debug_assertions)]
    fn locations_display(locations: &[&'static std::panic::Location<'static>]) -> String {
        locations
            .iter()
            .map(|location| format!(" - {location}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[derive(thiserror::Error, Debug)]
    pub enum UseSharedStateError {
        #[cfg_attr(
            debug_assertions,
            error(
                "[{0}] {1} is already borrowed at, so it cannot be borrowed mutably. Previous borrows:\n[{2}]\n\n",
                .source.attempted_at,
                .type_name,
                locations_display(&.source.already_borrowed_at)
            )
         )]
        #[cfg_attr(
            not(debug_assertions),
            error("{type_name} is already borrowed, so it cannot be borrowed mutably. (More detail available in debug mode)")
        )]
        AlreadyBorrowed {
            source: super::BorrowMutError,
            type_name: &'static str,
        },
        #[cfg_attr(
            debug_assertions,
            error(
                "[{0}] {1} is already borrowed mutably at [{2}], so it cannot be borrowed anymore.",
                .source.attempted_at,
                .type_name,
                locations_display(&.source.already_borrowed_at)
            )
         )]
        #[cfg_attr(
            not(debug_assertions),
            error("{type_name} is already borrowed mutably, so it cannot be borrowed anymore. (More detail available in debug mode)")
        )]
        AlreadyBorrowedMutably {
            source: super::BorrowError,
            type_name: &'static str,
        },
    }

    pub type UseSharedStateResult<T> = Result<T, UseSharedStateError>;
}

type ProvidedState<T> = Rc<RefCell<ProvidedStateInner<T>>>;

// Tracks all the subscribers to a shared State
pub(crate) struct ProvidedStateInner<T> {
    value: T,
    notify_any: Arc<dyn Fn(ScopeId)>,
    consumers: HashSet<ScopeId>,
    gen: usize,
}

impl<T> ProvidedStateInner<T> {
    pub(crate) fn notify_consumers(&mut self) {
        self.gen += 1;
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
/// # fn app() -> Element {
/// #     rsx! {
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
/// fn Parent<'a>(cx: Scope<'a>) -> Element {
///     use_shared_state_provider(|| Theme::Dark);
///     let theme = use_shared_state::<Theme>().unwrap();
///
///     rsx! {
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
/// fn Child<'a>(cx: Scope<'a>) -> Element {
///     let theme = use_shared_state::<Theme>().unwrap();
///     let current_theme = *theme.read();
///
///     rsx! {
///         match current_theme {
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
#[must_use]
pub fn use_shared_state<T: 'static>() -> Option<&UseSharedState<T>> {
    let state_owner: &mut Option<UseSharedStateOwner<T>> = &mut *cx.use_hook(move || {
        let scope_id = cx.scope_id();
        let root = cx.consume_context::<ProvidedState<T>>()?;

        root.borrow_mut().consumers.insert(scope_id);

        let state = UseSharedState::new(root);
        let owner = UseSharedStateOwner { state, scope_id };
        Some(owner)
    });
    state_owner.as_mut().map(|s| {
        s.state.gen = s.state.inner.borrow().gen;
        &s.state
    })
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
    gen: usize,
}

impl<T> UseSharedState<T> {
    fn new(inner: Rc<RefCell<ProvidedStateInner<T>>>) -> Self {
        let gen = inner.borrow().gen;
        Self { inner, gen }
    }

    /// Notify all consumers of the state that it has changed. (This is called automatically when you call "write")
    pub fn notify_consumers(&self) {
        self.inner.borrow_mut().notify_consumers();
    }

    /// Try reading the shared state
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn try_read(&self) -> UseSharedStateResult<Ref<'_, T>> {
        match self.inner.try_borrow() {
            Ok(value) => Ok(Ref::map(value, |inner| &inner.value)),
            Err(source) => Err(UseSharedStateError::AlreadyBorrowedMutably {
                source,
                type_name: std::any::type_name::<Self>(),
            }),
        }
    }

    /// Read the shared value
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn read(&self) -> Ref<'_, T> {
        match self.try_read() {
            Ok(value) => value,
            Err(message) => panic!(
                "Reading the shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Try writing the shared state
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn try_write(&self) -> UseSharedStateResult<RefMut<'_, T>> {
        match self.inner.try_borrow_mut() {
            Ok(mut value) => {
                value.notify_consumers();
                Ok(RefMut::map(value, |inner| &mut inner.value))
            }
            Err(source) => Err(UseSharedStateError::AlreadyBorrowed {
                source,
                type_name: std::any::type_name::<Self>(),
            }),
        }
    }

    /// Calling "write" will force the component to re-render
    ///
    ///
    // TODO: We prevent unncessary notifications only in the hook, but we should figure out some more global lock
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn write(&self) -> RefMut<'_, T> {
        match self.try_write() {
            Ok(value) => value,
            Err(message) => panic!(
                "Writing to shared state failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Tries writing the value without forcing a re-render
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn try_write_silent(&self) -> UseSharedStateResult<RefMut<'_, T>> {
        match self.inner.try_borrow_mut() {
            Ok(value) => Ok(RefMut::map(value, |inner| &mut inner.value)),
            Err(source) => Err(UseSharedStateError::AlreadyBorrowed {
                source,
                type_name: std::any::type_name::<Self>(),
            }),
        }
    }

    /// Writes the value without forcing a re-render
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn write_silent(&self) -> RefMut<'_, T> {
        match self.try_write_silent() {
            Ok(value) => value,
            Err(message) => panic!(
                "Writing to shared state silently failed: {}\n({:?})",
                message, message
            ),
        }
    }

    /// Take a reference to the inner value temporarily and produce a new value
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
        immutable_callback(&*self.read())
    }

    /// Take a mutable reference to the inner value temporarily and produce a new value
    #[cfg_attr(debug_assertions, track_caller)]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
        mutable_callback(&mut *self.write())
    }
}

impl<T> Clone for UseSharedState<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            gen: self.gen,
        }
    }
}

impl<T> PartialEq for UseSharedState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.gen == other.gen
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
/// # fn app() -> Element {
/// #     rsx! {
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
/// fn Parent<'a>(cx: Scope<'a>) -> Element {
///     use_shared_state_provider(|| Theme::Dark);
///     let theme = use_shared_state::<Theme>().unwrap();
///
///     rsx! {
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
pub fn use_shared_state_provider<T: 'static>(f: impl FnOnce() -> T) {
    cx.use_hook(|| {
        let state: ProvidedState<T> = Rc::new(RefCell::new(ProvidedStateInner {
            value: f(),
            notify_any: cx.schedule_update_any(),
            consumers: HashSet::new(),
            gen: 0,
        }));

        cx.provide_context(state);
    });
}
