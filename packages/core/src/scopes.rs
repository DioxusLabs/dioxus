use crate::{
    any_props::BoxedAnyProps, nodes::RenderReturn, runtime::Runtime, scope_context::Scope,
};
use std::{cell::Ref, fmt::Debug, rc::Rc};

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that acts a key for the internal slab of Scopes. This means that the key is not unqiue across
/// time. We do try and guarantee that between calls to `wait_for_work`, no ScopeIds will be recycled in order to give
/// time for any logic that relies on these IDs to properly update.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct ScopeId(pub usize);

impl ScopeId {
    /// The root ScopeId.
    ///
    /// This scope will last for the entire duration of your app, making it convenient for long-lived state
    /// that is created dynamically somewhere down the component tree.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// use dioxus_signals::*;
    /// let my_persistent_state = Signal::new_in_scope(ScopeId::ROOT, String::new());
    /// ```
    pub const ROOT: ScopeId = ScopeId(0);
}

/// A component's rendered state.
///
/// This state erases the type of the component's props. It is used to store the state of a component in the runtime.
pub struct ScopeState {
    pub(crate) runtime: Rc<Runtime>,
    pub(crate) context_id: ScopeId,
    pub(crate) last_rendered_node: Option<RenderReturn>,
    pub(crate) props: BoxedAnyProps,
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
    pub fn root_node(&self) -> &RenderReturn {
        self.try_root_node()
            .expect("The tree has not been built yet. Make sure to call rebuild on the tree before accessing its nodes.")
    }

    /// Try to get a handle to the currently active head node arena for this Scope
    ///
    /// This is useful for traversing the tree outside of the VirtualDom, such as in a custom renderer or in SSR.
    ///
    /// Returns [`None`] if the tree has not been built yet.
    pub fn try_root_node(&self) -> Option<&RenderReturn> {
        self.last_rendered_node.as_ref()
    }

    pub(crate) fn state(&self) -> Ref<'_, Scope> {
        self.runtime.get_state(self.context_id).unwrap()
    }
}
