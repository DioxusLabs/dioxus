use std::{any::Any, rc::Rc};

use crate::{
    RenderTargetId, diff::context::DiffContext, innerlude::*, mutations::append_children_to,
    render_driver::RenderDriver,
};

/// Properties for the [`Portal()`] component.
#[derive(Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub struct PortalProps {
    target: RenderTargetId,
    /// The children rendered into the portal target.
    children: LastRenderedNode,
}

impl PortalProps {
    #[allow(dead_code, clippy::type_complexity)]
    fn builder() -> PortalPropsBuilder<((), ())> {
        PortalPropsBuilder {
            fields: ((), ()),
            _phantom: (),
        }
    }
}

#[must_use]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct PortalPropsBuilder<TypedBuilderFields> {
    fields: TypedBuilderFields,
    _phantom: (),
}

impl Properties for PortalProps {
    type Builder = PortalPropsBuilder<((), ())>;
    type ComponentBuilder<RenderFn, Marker> =
        ComponentBuilder<RenderFn, Self::Builder, Self, Marker>;

    fn builder() -> Self::Builder {
        PortalProps::builder()
    }

    fn component_builder<RenderFn, Marker>(
        render_fn: RenderFn,
    ) -> Self::ComponentBuilder<RenderFn, Marker> {
        ComponentBuilder::new(render_fn, Self::builder())
    }

    fn into_vcomponent<M: 'static>(self, render_fn: impl ComponentFunction<Self, M>) -> VComponent {
        let type_name = std::any::type_name_of_val(&render_fn);
        let render_fn_ptr = render_fn.fn_ptr();
        let props = Box::new(VProps::new(
            render_fn,
            <Self as Properties>::memoize,
            self,
            type_name,
        ));
        VComponent::new_with_driver(type_name, render_fn_ptr, Rc::new(PortalDriver), props)
    }

    fn memoize(&mut self, new: &Self) -> bool {
        // Unconditionally adopt the new props' fields. Each `rsx!` macro
        // expansion produces fresh `Rc<VNodeInner>` instances even for
        // identical markup, so the `self == new` short-circuit on
        // `Rc::ptr_eq` is effectively unreachable in practice — we still
        // return the equality flag so callers can skip a redundant diff
        // when the pointers happen to alias (e.g. cached test fixtures).
        let equal = self == new;
        self.target = new.target;
        self.children = new.children.clone();
        equal
    }
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> PortalPropsBuilder<((), __children)> {
    #[allow(clippy::type_complexity)]
    pub fn target(
        self,
        target: RenderTargetId,
    ) -> PortalPropsBuilder<((RenderTargetId,), __children)> {
        let (_, children) = self.fields;
        PortalPropsBuilder {
            fields: ((target,), children),
            _phantom: self._phantom,
        }
    }
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__target> PortalPropsBuilder<(__target, ())> {
    #[allow(clippy::type_complexity)]
    pub fn children(self, children: Element) -> PortalPropsBuilder<(__target, (Element,))> {
        let (target, _) = self.fields;
        PortalPropsBuilder {
            fields: (target, (children,)),
            _phantom: self._phantom,
        }
    }
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
impl PortalPropsBuilder<((RenderTargetId,), (Element,))> {
    pub fn build(self) -> PortalProps {
        let (target, children) = self.fields;
        PortalProps {
            target: target.0,
            children: LastRenderedNode::new(children.0),
        }
    }
}

/// Render children into another renderer target while keeping their logical parent.
///
/// ## Details
///
/// A portal changes where renderer mutations for its children are written. It does not change
/// component ownership, context, or event propagation. Children rendered through a portal can still
/// consume context from their logical ancestors, and events from the portal target bubble through
/// the same Dioxus component tree.
///
/// Each render target has its own [`ElementId`] arena. Hosts create targets with
/// [`Runtime::create_render_target`] and serve them with a [`MultiWriter`]. A single
/// [`WriteMutations`] writer automatically serves the root target; hosts with several targets can
/// pass a `BTreeMap<RenderTargetId, W>` or their own [`MultiWriter`] implementation. If the host is
/// not currently serving the target, renderer mutations for that target are skipped.
///
/// ## Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// #[component]
/// fn App(target: RenderTargetId) -> Element {
///     rsx! {
///         main { "Main view" }
///         Portal {
///             target,
///             aside { "Rendered in another target" }
///         }
///     }
/// }
/// ```
///
/// ## Usage
///
/// Use `Portal` when a renderer exposes more than one target, such as a desktop child window,
/// overlay root, or another renderer-owned mount point. For ordinary layout within the current
/// target, render children directly instead.
#[allow(non_snake_case)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn Portal(__props: PortalProps) -> Element {
    unreachable!("Portal should not be called directly")
}

/// The rendering lifecycle of a portal scope: its output lives at the root of another render
/// target instead of mounting at the scope's slot.
struct PortalDriver;

fn portal_props(dom: &VirtualDom, scope_id: ScopeId) -> (RenderTargetId, LastRenderedNode) {
    let props = dom.scopes[scope_id.index()]
        .props
        .props()
        .downcast_ref::<PortalProps>()
        .expect("portal scope carries PortalProps");
    (props.target, props.children.clone())
}

/// Create `children` inside `target_id`, record them as the scope's
/// rendered output, and fire mount lifecycle when writes are enabled.
/// Shared by initial creation and the retarget arm of `diff`.
fn mount_children(
    scope_id: ScopeId,
    target_id: RenderTargetId,
    children: LastRenderedNode,
    parent: Option<MountRef>,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) {
    debug_assert_eq!(
        dom.runtime.current_render_target_id(),
        target_id,
        "portal mount runs inside the portal scope, whose target_id routes its writes"
    );
    let mut render_to = to;
    let should_mount = render_to.is_some();
    let mut root_mount = None;
    if let Some(to) = render_to.as_deref_mut() {
        append_children_to(to, ElementId::ROOT, dom.runtime.clone(), |to| {
            let created = dom.create_children_with_parents(
                Some(to),
                std::slice::from_ref(children.as_vnode()),
                None,
                parent,
            );
            root_mount = created.mounts.first().copied();
            created.nodes
        });
    } else {
        let created = dom.create_children_with_parents(
            None,
            std::slice::from_ref(children.as_vnode()),
            None,
            parent,
        );
        root_mount = created.mounts.first().copied();
    }
    dom.scopes[scope_id.index()].last_rendered_node = Some(MountedOutput::new(
        children,
        root_mount.expect("portal children should create a root mount"),
    ));
    if should_mount {
        dom.runtime.get_state(scope_id).mount(&dom.runtime);
    }
}

fn remount_children(
    scope_id: ScopeId,
    target_id: RenderTargetId,
    children: LastRenderedNode,
    root_mount: MountId,
    parent: Option<MountRef>,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) {
    debug_assert_eq!(
        dom.runtime.current_render_target_id(),
        target_id,
        "portal remount runs inside the portal scope, whose target_id routes its writes"
    );
    let mut render_to = to;
    let should_mount = render_to.is_some();
    if let Some(to) = render_to.as_deref_mut() {
        append_children_to(to, ElementId::ROOT, dom.runtime.clone(), |to| {
            children
                .as_vnode()
                .recreate_with_mount(dom, root_mount, None, parent, Some(to))
                .nodes
        });
    } else {
        children
            .as_vnode()
            .recreate_with_mount(dom, root_mount, None, parent, None);
    }
    dom.scopes[scope_id.index()].last_rendered_node =
        Some(MountedOutput::new(children, root_mount));
    if should_mount {
        dom.runtime.get_state(scope_id).mount(&dom.runtime);
    }
}

impl RenderDriver for PortalDriver {
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
        if new {
            let (target_id, children) = portal_props(dom, scope_id);
            // The scope was allocated with its parent's target; declare it as
            // a retargeting point before anything mounts under it. Later
            // target changes are applied by the retarget arm of `diff`, which
            // must observe the old target first.
            dom.runtime.set_scope_target_id(scope_id, target_id);

            dom.runtime.clone().with_scope_on_stack(scope_id, || {
                mount_children(scope_id, target_id, children, parent, dom, to);
                0
            })
        } else {
            // Re-creating a live scope: the props' children handle is not
            // mount-accurate (mounts land on the clone the first create
            // rendered), so re-create from the mounted output and the scope's
            // current target. Pending prop changes apply on the next `diff`.
            let old_output = dom.scopes[scope_id.index()]
                .last_rendered_node
                .clone()
                .expect("portal scope must have rendered before re-create");
            let target_id = dom.runtime.get_state(scope_id).target_id();
            let root_mount = old_output.root_mount();
            let children = old_output.node().clone();

            dom.runtime.clone().with_scope_on_stack(scope_id, || {
                remount_children(scope_id, target_id, children, root_mount, parent, dom, to);
                0
            })
        }
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        _parent_context: Option<DiffContext<'_>>,
        mut to: Option<&mut (dyn WriteMutations + '_)>,
    ) {
        let (target_id, new_children) = portal_props(dom, scope_id);

        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let old_children = dom.scopes[scope_id.index()]
                .last_rendered_node
                .take()
                .unwrap();
            let old_target_id = dom.runtime.get_state(scope_id).target_id();

            if old_target_id != target_id {
                let old_mount = old_children.root_mount();
                let logical_parent = dom.mounted_logical_parent(old_mount);

                old_children
                    .as_vnode()
                    .remove_node_inner(old_mount, dom, to.as_deref_mut(), true);

                // Ordering is correctness-critical: writes route through the
                // portal scope's `target_id`, so the removal above resolves
                // against the old target and `mount_children` below resolves
                // against the new one.
                dom.runtime.set_scope_target_id(scope_id, target_id);

                mount_children(
                    scope_id,
                    target_id,
                    new_children,
                    logical_parent,
                    dom,
                    to.as_deref_mut(),
                );
                return;
            }

            let mut render_to = to.filter(|_| dom.runtime.scope_should_render(scope_id));
            let new_mount = old_children.mounted_vnode().diff_node(
                new_children.as_vnode(),
                dom,
                render_to.as_deref_mut(),
            );
            dom.scopes[scope_id.index()].last_rendered_node =
                Some(MountedOutput::new(new_children, new_mount));
            if render_to.is_some() {
                dom.runtime.get_state(scope_id).mount(&dom.runtime);
            }
        })
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
    ) {
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let mut render_to = to;
            // `PortalDriver::create` always sets `last_rendered_node` before
            // returning, and removal only fires after a scope has gone
            // through `create`, so the clone is always `Some`.
            let node = dom.scopes[scope_id.index()]
                .last_rendered_node
                .as_ref()
                .cloned()
                .expect("portal scope must have rendered before remove");
            node.as_vnode().remove_node_inner(
                node.root_mount(),
                dom,
                render_to.as_deref_mut(),
                destroy_component_state,
            );
        });

        if destroy_component_state {
            dom.drop_scope(scope_id);
        }
    }
}
