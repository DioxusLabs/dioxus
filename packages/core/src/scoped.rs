use crate::ScopeState;

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that acts a key for the internal slab of Scopes. This means that the key is not unqiue across
/// time. We do try and guarantee that between calls to `wait_for_work`, no ScopeIds will be recycled in order to give
/// time for any logic that relies on these IDs to properly update.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

/// A wrapper around the [`Scoped`] object that contains a reference to the [`ScopeState`] and properties for a given
/// component.
///
/// The [`Scope`] is your handle to the [`crate::VirtualDom`] and the component state. Every component is given its own
/// [`ScopeState`] and merged with its properties to create a [`Scoped`].
///
/// The [`Scope`] handle specifically exists to provide a stable reference to these items for the lifetime of the
/// component render.
pub type Scope<'a, T = ()> = &'a Scoped<'a, T>;

// This ScopedType exists because we want to limit the amount of monomorphization that occurs when making inner
// state type generic over props. When the state is generic, it causes every method to be monomorphized for every
// instance of Scope<T> in the codebase.
//
//
/// A wrapper around a component's [`ScopeState`] and properties. The [`ScopeState`] provides the majority of methods
/// for the VirtualDom and component state.
pub struct Scoped<'a, T = ()> {
    /// The component's state and handle to the scheduler.
    ///
    /// Stores things like the custom bump arena, spawn functions, hooks, and the scheduler.
    pub scope: &'a ScopeState,

    /// The component's properties.
    pub props: &'a T,
}

impl<'a, T> std::ops::Deref for Scoped<'a, T> {
    type Target = &'a ScopeState;
    fn deref(&self) -> &Self::Target {
        &self.scope
    }
}
