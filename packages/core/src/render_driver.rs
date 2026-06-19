use std::any::Any;

use crate::{
    WriteMutations,
    diff::context::DiffContext,
    innerlude::MountRef,
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// A scope's rendering lifecycle.
///
/// Every scope owns exactly one driver, attached when its [`VComponent`] is constructed and fixed
/// for the scope's lifetime. Plain components use [`BodyDriver`], which renders the type-erased
/// props stored in [`ScopeState`]. Portal and suspense attach drivers that manage
/// `last_rendered_node` directly with no component body to run.
///
/// [`VComponent`]: crate::nodes::VComponent
/// [`ScopeState`]: crate::scopes::ScopeState
pub(crate) trait RenderDriver: 'static {
    /// The driver as `Any`, for specialized driver downcasts.
    fn as_any(&self) -> &dyn Any;

    /// Whether `other` renders the same special lifecycle as this driver.
    ///
    /// Plain body components are additionally identified by `VComponent::render_fn` before this
    /// hook is consulted.
    fn same_component(&self, other: &dyn RenderDriver) -> bool {
        self.as_any().type_id() == other.as_any().type_id()
    }

    /// Mount this scope's output. `new` is true when the scope was allocated for this create and
    /// has never run or rendered. Maintains `scopes[id].last_rendered_node`. Returns the number of
    /// nodes left on the renderer stack.
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<MountRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> usize;

    /// Diff this scope's output against its current props.
    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        parent_context: Option<DiffContext<'_>>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    );

    /// Remove this scope's output. When `destroy_component_state` is false the output is only being
    /// lifted out of the real DOM and the driver must keep component state alive.
    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
    );
}

/// Remove a scope's rendered output from the DOM, and drop the scope when
/// `destroy_component_state` is set. Shared by [`BodyDriver`] and drivers whose output is removed
/// the same way.
pub(crate) fn remove_rendered_output(
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    to: Option<&mut (dyn WriteMutations + '_)>,
    destroy_component_state: bool,
) {
    let node = dom.scopes[scope_id.index()]
        .last_rendered_node
        .clone()
        .expect("scope being removed should have last_rendered_node set");
    node.as_vnode()
        .remove_node_inner(node.root_mount(), dom, to, destroy_component_state);

    if destroy_component_state {
        dom.drop_scope(scope_id);
    }
}

/// The rendering lifecycle of a plain component.
pub(crate) struct BodyDriver;

impl RenderDriver for BodyDriver {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<MountRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> usize {
        let new_node = if new {
            LastRenderedNode::new(dom.run_scope(scope_id))
        } else {
            dom.scopes[scope_id.index()]
                .last_rendered_node
                .as_ref()
                .expect("Component to be mounted")
                .node()
                .clone()
        };

        dom.mark_clean(scope_id);
        dom.create_scope(to, scope_id, new_node, parent)
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        parent_context: Option<DiffContext<'_>>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) {
        let body = dom.run_scope(scope_id);
        dom.diff_scope(to, scope_id, body, parent_context);
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
    ) {
        remove_rendered_output(dom, scope_id, to, destroy_component_state);
    }
}
