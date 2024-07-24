use crate::{
    any_props::BoxedAnyProps, nodes::RenderReturn, reactive_context::ReactiveContext,
    scope_context::Scope, Runtime, VNode,
};
use std::{cell::Ref, rc::Rc};

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that acts a key for the internal slab of Scopes. This means that the key is not unique across
/// time. We do try and guarantee that between calls to `wait_for_work`, no ScopeIds will be recycled in order to give
/// time for any logic that relies on these IDs to properly update.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

impl std::fmt::Debug for ScopeId {
    #[allow(unused_mut)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_tuple("ScopeId");
        let mut builder = builder.field(&self.0);
        #[cfg(debug_assertions)]
        {
            if let Some(name) = Runtime::current()
                .ok()
                .as_ref()
                .and_then(|rt| rt.get_state(*self))
            {
                builder = builder.field(&name.name);
            }
        }
        builder.finish()
    }
}

impl ScopeId {
    /// The ScopeId of the main scope passed into [`VirtualDom::new`].
    ///
    /// This scope will last for the entire duration of your app, making it convenient for long-lived state
    /// that is created dynamically somewhere down the component tree.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// let my_persistent_state = Signal::new_in_scope(String::new(), ScopeId::APP);
    /// ```
    // ScopeId(0) is the root scope wrapper
    // ScopeId(1) is the default error boundary
    // ScopeId(2) is the default suspense boundary
    // ScopeId(3) is the users root scope
    pub const APP: ScopeId = ScopeId(3);

    /// The ScopeId of the topmost scope in the tree.
    /// This will be higher up in the tree than [`ScopeId::APP`] because dioxus inserts a default [`SuspenseBoundary`] and [`ErrorBoundary`] at the root of the tree.
    // ScopeId(0) is the root scope wrapper
    pub const ROOT: ScopeId = ScopeId(0);

    pub(crate) const PLACEHOLDER: ScopeId = ScopeId(usize::MAX);

    pub(crate) fn is_placeholder(&self) -> bool {
        *self == Self::PLACEHOLDER
    }
}

/// A component's rendered state.
///
/// This state erases the type of the component's props. It is used to store the state of a component in the runtime.
pub struct ScopeState {
    pub(crate) runtime: Rc<Runtime>,
    pub(crate) context_id: ScopeId,
    /// The last node that has been rendered for this component. This node may not ben mounted
    /// During suspense, this component can be rendered in the background multiple times
    pub(crate) last_rendered_node: Option<RenderReturn>,
    pub(crate) props: BoxedAnyProps,
    pub(crate) reactive_context: ReactiveContext,
}

impl Drop for ScopeState {
    fn drop(&mut self) {
        self.runtime.remove_scope(self.context_id);
    }
}

impl ScopeState {
    /// Get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Panics if the tree has not been built yet.
    pub fn root_node(&self) -> &VNode {
        self.try_root_node()
            .expect("The tree has not been built yet. Make sure to call rebuild on the tree before accessing its nodes.")
    }

    /// Try to get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Returns [`None`] if the tree has not been built yet.
    pub fn try_root_node(&self) -> Option<&VNode> {
        self.last_rendered_node.as_deref()
    }

    /// Returns the scope id of this [`ScopeState`].
    pub fn id(&self) -> ScopeId {
        self.context_id
    }

    pub(crate) fn state(&self) -> Ref<'_, Scope> {
        self.runtime.get_state(self.context_id).unwrap()
    }
}
