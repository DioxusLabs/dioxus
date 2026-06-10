use std::rc::Rc;

use crate::{
    Element, WriteMutations,
    diff::context::DiffContext,
    innerlude::{ElementRef, ScopeOrder},
    scopes::{LastRenderedNode, ScopeId},
    virtual_dom::VirtualDom,
};

/// A scope's rendering lifecycle.
///
/// Every scope owns exactly one driver: plain components use
/// [`ComponentDriver`], which mounts and diffs the element returned by the
/// scope's body, while portal and suspense scopes register drivers (during
/// their body run) that manage the scope's `last_rendered_node` themselves and
/// treat the body element as an empty placeholder.
///
/// Dispatch invokes drivers with no scope pushed on the runtime stack for the
/// call; drivers push their own scope/suspense frames.
pub(crate) trait RenderDriver {
    /// Mount this scope's output. `body` is the element the scope's body just
    /// returned, or `None` when re-creating a live scope whose body was not
    /// re-run. Returns the number of nodes left on the renderer stack.
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        body: Option<Element>,
        parent: Option<ElementRef>,
        to: Option<&mut dyn WriteMutations>,
    ) -> usize;

    /// Diff this scope's output. `body` is the element the scope's body just
    /// returned; `to` is the ungated writer and the driver applies its own
    /// write gating.
    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        body: Element,
        parent_context: Option<DiffContext<'_>>,
        to: Option<&mut dyn WriteMutations>,
    );

    /// Remove this scope's output. When `destroy_component_state` is false
    /// the output is only being lifted out of the real DOM and the driver
    /// must keep component state alive.
    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    );
}

/// A driver-managed scope's body exists only to run hooks; it must render the
/// empty placeholder so the discarded body element can never own real output.
pub(crate) fn debug_assert_driver_body_is_empty(body: &Element) {
    debug_assert!(
        matches!(
            body,
            Ok(node) if node.template == crate::nodes::VNode::placeholder().template
                && matches!(
                    &*node.dynamic_nodes,
                    [crate::innerlude::DynamicNode::Fragment(roots)] if roots.is_empty()
                )
        ),
        "a scope with a render driver must return an empty element from its body"
    );
}

/// The shared driver instance for plain components.
pub(crate) fn component_driver() -> Rc<dyn RenderDriver> {
    thread_local! {
        static COMPONENT: Rc<ComponentDriver> = Rc::new(ComponentDriver);
    }
    COMPONENT.with(|driver| driver.clone() as Rc<dyn RenderDriver>)
}

/// The rendering lifecycle of a plain component: the element returned by the
/// scope's body is the scope's rendered output.
pub(crate) struct ComponentDriver;

impl RenderDriver for ComponentDriver {
    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        body: Option<Element>,
        parent: Option<ElementRef>,
        mut to: Option<&mut dyn WriteMutations>,
    ) -> usize {
        if let Some(body) = body {
            dom.scopes[scope_id.0].last_rendered_node = Some(LastRenderedNode::new(body));
        }

        // If our scope landed in `dirty_scopes` during its initial render
        // (e.g. a hook synchronously queued an update for itself), drain the
        // entry now so we don't re-process the same scope after creation.
        let height = dom.runtime.get_state(scope_id).height;
        dom.dirty_scopes.remove(&ScopeOrder::new(height, scope_id));

        let new_node = dom.scopes[scope_id.0]
            .last_rendered_node
            .clone()
            .expect("Component to be mounted");

        dom.create_scope(to.as_mut(), scope_id, new_node, parent)
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        body: Element,
        parent_context: Option<DiffContext<'_>>,
        mut to: Option<&mut dyn WriteMutations>,
    ) {
        dom.diff_scope(to.as_mut(), scope_id, body, parent_context);
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        mut to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    ) {
        // Removal only fires after the scope has rendered at least once via
        // `create`, which sets `last_rendered_node` to `Some`. A scope that
        // never rendered is dropped without going through this path.
        let node = dom.scopes[scope_id.0]
            .last_rendered_node
            .clone()
            .expect("scope being removed should have last_rendered_node set");
        node.remove_node_inner(dom, to.as_mut(), destroy_component_state);

        if destroy_component_state {
            // Now drop all the resources
            dom.drop_scope(scope_id);
        }
    }
}
