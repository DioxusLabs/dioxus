use crate::{
    WriteMutations,
    innerlude::ElementRef,
    scope_context::SuspenseLocation,
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// The rendering lifecycle driver for a scope.
///
/// Every scope owns exactly one driver:
/// - Plain components use [`BodyDriver`].
/// - Custom components may use specialized drivers, such as
///   [`SuspenseDriver`](crate::suspense::SuspenseDriver).
pub(crate) trait RenderDriver: 'static {
    /// The suspense location to store on a newly-created scope owned by this
    /// driver.
    fn initial_suspense_location(&self, parent: SuspenseLocation) -> SuspenseLocation {
        parent
    }

    /// Mount this scope's output. `to` receives DOM mutations; pass `None` for
    /// background rendering (e.g. suspended children).
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> usize;

    /// Diff this scope's output against its current props.
    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
    );

    /// Remove this scope's output.
    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    );
}

/// The concrete driver for plain (non-suspense) components.
pub(crate) struct BodyDriver;

impl BodyDriver {
    pub fn new() -> BodyDriver {
        BodyDriver
    }
}

impl RenderDriver for BodyDriver {
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> usize {
        if new {
            let body = dom.run_scope(scope_id);
            dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(body));
        }
        let new_node = dom.scopes[scope_id.0]
            .last_rendered_node
            .clone()
            .expect("Component to be mounted");
        dom.create_scope(to, scope_id, new_node, parent)
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) {
        let body = dom.run_scope(scope_id);
        dom.diff_scope(to, scope_id, body);
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        if let Some(node) = dom.scopes[scope_id.0].last_rendered_node.clone() {
            node.remove_node_inner(dom, to, destroy_component_state, replace_with)
        };

        if destroy_component_state {
            dom.drop_scope(scope_id);
        }
    }
}
