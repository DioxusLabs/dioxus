use crate::{
    any_props::BoxedAnyProps, reactive_context::ReactiveContext, scope_context::Scope, Element,
    RenderError, Runtime, VNode,
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
            if let Some(scope) = Runtime::try_current()
                .as_ref()
                .and_then(|r| r.try_get_state(*self))
            {
                builder = builder.field(&scope.name);
            }
        }
        builder.finish()
    }
}

impl ScopeId {
    /// The ScopeId of the main scope passed into [`crate::VirtualDom::new`].
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
    // Refer to `RootScopeWrapper` (root_wrapper.rs) to see where these constants come from.
    // ScopeId(0) is the root scope wrapper
    // ScopeId(1) is the default suspense boundary
    // ScopeId(2) is the default error boundary
    // ScopeId(3) is the users root scope
    pub const APP: ScopeId = ScopeId(3);

    /// The ScopeId of the topmost error boundary in the tree.
    pub const ROOT_ERROR_BOUNDARY: ScopeId = ScopeId(2);

    /// The ScopeId of the topmost suspense boundary in the tree.
    pub const ROOT_SUSPENSE_BOUNDARY: ScopeId = ScopeId(1);

    /// The ScopeId of the topmost scope in the tree.
    /// This will be higher up in the tree than [`ScopeId::APP`] because dioxus inserts a default [`crate::SuspenseBoundary`] and [`crate::ErrorBoundary`] at the root of the tree.
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
    pub(crate) last_rendered_node: Option<LastRenderedNode>,
    pub(crate) props: BoxedAnyProps,
    pub(crate) reactive_context: ReactiveContext,
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
        match &self.last_rendered_node {
            Some(LastRenderedNode::Real(vnode)) => Some(vnode),
            Some(LastRenderedNode::Placeholder(vnode, _)) => Some(vnode),
            None => None,
        }
    }

    /// Returns the scope id of this [`ScopeState`].
    pub fn id(&self) -> ScopeId {
        self.context_id
    }

    pub(crate) fn state(&self) -> Ref<'_, Scope> {
        self.runtime.get_state(self.context_id)
    }

    /// Returns the height of this scope in the tree.
    pub fn height(&self) -> u32 {
        self.state().height()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum LastRenderedNode {
    Real(VNode),
    Placeholder(VNode, RenderError),
}

impl std::ops::Deref for LastRenderedNode {
    type Target = VNode;

    fn deref(&self) -> &Self::Target {
        match self {
            LastRenderedNode::Real(vnode) => vnode,
            LastRenderedNode::Placeholder(vnode, _err) => vnode,
        }
    }
}

impl LastRenderedNode {
    pub fn new(node: Element) -> Self {
        match node {
            Ok(vnode) => LastRenderedNode::Real(vnode),
            Err(err) => LastRenderedNode::Placeholder(VNode::placeholder(), err),
        }
    }

    pub fn as_vnode(&self) -> &VNode {
        match self {
            LastRenderedNode::Real(vnode) => vnode,
            LastRenderedNode::Placeholder(vnode, _err) => vnode,
        }
    }
}

impl Drop for ScopeState {
    fn drop(&mut self) {
        self.runtime.remove_scope(self.context_id);
    }
}
