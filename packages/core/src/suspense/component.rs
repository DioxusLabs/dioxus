use crate::{
    DynamicNode,
    innerlude::*,
    mount::{RenderMode, SuspenseBranch},
    scope_context::SuspenseLocation,
};

/// Properties for the [`SuspenseBoundary()`] component.
#[derive(Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub struct SuspenseBoundaryProps {
    fallback: Callback<SuspenseContext, Element>,
    /// The children of the suspense boundary
    children: LastRenderedNode,
}

impl SuspenseBoundaryProps {
    /**
    Create a builder for building `SuspenseBoundaryProps`.
    On the builder, call `.fallback(...)`, `.children(...)`(optional) to set the values of the fields.
    Finally, call `.build()` to create the instance of `SuspenseBoundaryProps`.
                        */
    #[allow(dead_code, clippy::type_complexity)]
    fn builder() -> SuspenseBoundaryPropsBuilder<((), ())> {
        SuspenseBoundaryPropsBuilder {
            owner: Owner::default(),
            fields: ((), ()),
            _phantom: (),
        }
    }
}
#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct SuspenseBoundaryPropsBuilder<TypedBuilderFields> {
    owner: Owner,
    fields: TypedBuilderFields,
    _phantom: (),
}
impl Properties for SuspenseBoundaryProps {
    type Builder = SuspenseBoundaryPropsBuilder<((), ())>;
    fn builder() -> Self::Builder {
        SuspenseBoundaryProps::builder()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        self.fallback.__point_to(&new.fallback);
        self.children = new.children.clone();
        false
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub trait SuspenseBoundaryPropsBuilder_Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}
impl<T> SuspenseBoundaryPropsBuilder_Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}
impl<T> SuspenseBoundaryPropsBuilder_Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> SuspenseBoundaryPropsBuilder<((), __children)> {
    #[allow(clippy::type_complexity)]
    pub fn fallback<__Marker>(
        self,
        fallback: impl SuperInto<Callback<SuspenseContext, Element>, __Marker>,
    ) -> SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)> {
        let owner = self.owner.clone();
        let fallback = (with_owner(owner, move || fallback.super_into()),);
        let (_, children) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, children),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum SuspenseBoundaryPropsBuilder_Error_Repeated_field_fallback {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)> {
    #[deprecated(note = "Repeated field fallback")]
    #[allow(clippy::type_complexity)]
    pub fn fallback(
        self,
        _: SuspenseBoundaryPropsBuilder_Error_Repeated_field_fallback,
    ) -> SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)> {
        self
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__fallback> SuspenseBoundaryPropsBuilder<(__fallback, ())> {
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        children: Element,
    ) -> SuspenseBoundaryPropsBuilder<(__fallback, (Element,))> {
        let (fallback, _) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, (children,)),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum SuspenseBoundaryPropsBuilder_Error_Repeated_field_children {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__fallback> SuspenseBoundaryPropsBuilder<(__fallback, (Element,))> {
    #[deprecated(note = "Repeated field children")]
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        _: SuspenseBoundaryPropsBuilder_Error_Repeated_field_children,
    ) -> SuspenseBoundaryPropsBuilder<(__fallback, (Element,))> {
        self
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum SuspenseBoundaryPropsBuilder_Error_Missing_required_field_fallback {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
impl<__children> SuspenseBoundaryPropsBuilder<((), __children)> {
    #[deprecated(note = "Missing required field fallback")]
    pub fn build(
        self,
        _: SuspenseBoundaryPropsBuilder_Error_Missing_required_field_fallback,
    ) -> SuspenseBoundaryProps {
        panic!()
    }
}
#[doc(hidden)]
#[derive(Clone)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
pub struct SuspenseBoundaryPropsWithOwner {
    inner: SuspenseBoundaryProps,
    owner: Owner,
}
impl PartialEq for SuspenseBoundaryPropsWithOwner {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl SuspenseBoundaryPropsWithOwner {
    /// Create a component from the props.
    pub fn into_vcomponent<M: 'static>(
        self,
        render_fn: impl ComponentFunction<SuspenseBoundaryProps, M>,
    ) -> VComponent {
        let component_name = std::any::type_name_of_val(&render_fn);
        VComponent::new(
            move |wrapper: Self| render_fn.rebuild(wrapper.inner),
            self,
            component_name,
        )
    }
}
impl Properties for SuspenseBoundaryPropsWithOwner {
    type Builder = ();
    fn builder() -> Self::Builder {
        unreachable!()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        self.inner.memoize(&new.inner)
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children: SuspenseBoundaryPropsBuilder_Optional<Element>>
    SuspenseBoundaryPropsBuilder<((Callback<SuspenseContext, Element>,), __children)>
{
    pub fn build(self) -> SuspenseBoundaryPropsWithOwner {
        let (fallback, children) = self.fields;
        SuspenseBoundaryPropsWithOwner {
            inner: SuspenseBoundaryProps {
                fallback: fallback.0,
                children: LastRenderedNode::new(children.into_value(VNode::empty)),
            },
            owner: self.owner,
        }
    }
}
/// Suspense Boundaries let you render a fallback UI while a child component is suspended.
///
/// # Example
///
/// ```rust
/// # use dioxus::prelude::*;
/// # fn Article() -> Element { rsx! { "Article" } }
/// fn App() -> Element {
///     rsx! {
///         SuspenseBoundary {
///             fallback: |_| rsx! { "Loading..." },
///             Article {}
///         }
///     }
/// }
/// ```
#[allow(non_snake_case)]
pub fn SuspenseBoundary(__props: SuspenseBoundaryProps) -> Element {
    unreachable!("SuspenseBoundary should not be called directly")
}
#[allow(non_snake_case)]
#[doc(hidden)]
mod SuspenseBoundary_completions {
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        SuspenseBoundary {},
    }
}
#[allow(unused)]
pub use SuspenseBoundary_completions::Component::SuspenseBoundary;
use generational_box::Owner;

/// Suspense has a custom diffing algorithm that diffs the suspended nodes in the background without rendering them
impl SuspenseBoundaryProps {
    /// Try to downcast [`AnyProps`] to [`SuspenseBoundaryProps`]
    pub(crate) fn downcast_from_props(props: &mut dyn AnyProps) -> Option<&mut Self> {
        let inner: Option<&mut SuspenseBoundaryPropsWithOwner> = props.props_mut().downcast_mut();
        inner.map(|inner| &mut inner.inner)
    }

    pub(crate) fn create<M: WriteMutations>(
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
    ) -> usize {
        let mut scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
        // If the ScopeId is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
        if scope_id.is_placeholder() {
            {
                let suspense_context = SuspenseContext::new();
                let scope_state = dom
                    .new_scope(component.props.duplicate(), component.name)
                    .state();
                scope_state.set_suspense_boundary(suspense_context.clone());
                suspense_context.mount(scope_state.id);
                scope_id = scope_state.id;
            }

            // Store the scope id for the next render
            dom.set_mounted_dyn_node(mount, idx, scope_id.0);
        }
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope_state = &mut dom.scopes[scope_id.0];
            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
            let suspense_context =
                SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                    .unwrap();

            let children = props.children.clone();

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create(dom, parent, None::<&mut M>);
            });

            store_suspense_children(dom, scope_id, &children);

            // If there are suspended futures, render the fallback

            if !suspense_context.suspended_futures().is_empty() {
                let placeholder_context = suspense_context.clone();
                let (node, nodes_created) =
                    suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                        let fallback = {
                            let scope_state = &mut dom.scopes[scope_id.0];
                            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
                            props.fallback
                        };
                        let branch = SuspenseBranch::new(children.as_vnode().clone());
                        store_suspended_branch(dom, scope_id, &branch);
                        placeholder_context.set_suspended_branch(branch);
                        let suspense_placeholder =
                            LastRenderedNode::new(fallback.call(placeholder_context));
                        let nodes_created = suspense_placeholder.create(dom, parent, to);
                        (suspense_placeholder, nodes_created)
                    });

                dom.scopes[scope_id.0].last_rendered_node = Some(node);
                nodes_created
            } else {
                // Otherwise just render the children in the real dom
                debug_assert!(children.mount.get().mounted());
                let nodes_created = suspense_context
                    .under_suspense_boundary(&dom.runtime(), || children.create(dom, parent, to));
                dom.scopes[scope_id.0].last_rendered_node = Some(children);
                suspense_context.take_suspended_branch();
                mark_suspense_resolved(&suspense_context, dom, scope_id);

                nodes_created
            }
        })
    }

    #[doc(hidden)]
    /// Manually rerun the children of this suspense boundary without diffing against the old nodes.
    ///
    /// This should only be called by dioxus-web after the suspense boundary has been streamed in from the server.
    pub fn resolve_suspense<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        to: &mut M,
        only_write_templates: impl FnOnce(&mut M),
        replace_with: usize,
    ) {
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let _runtime = RuntimeGuard::new(dom.runtime());
            let Some(scope_state) = dom.scopes.get_mut(scope_id.0) else {
                return;
            };

            // Reset the suspense context
            let suspense_context = scope_state.state().suspense_boundary().unwrap().clone();
            suspense_context.inner.suspended_tasks.borrow_mut().clear();

            // Get the parent of the suspense boundary to later create children with the right parent
            let currently_rendered = scope_state.last_rendered_node.clone().unwrap();
            let mount = currently_rendered.mount.get();
            let parent = dom
                .runtime
                .mounts
                .borrow()
                .get(mount.0)
                .expect("suspense placeholder is not mounted")
                .render_parent;

            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();

            // Unmount any children to reset any scopes under this suspense boundary
            let children = props.children.clone();
            // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
            if let Some(branch) = suspense_context.take_suspended_branch() {
                branch.into_root().remove_node(&mut *dom, None::<&mut M>);
            }

            // Streaming has pre-pushed `replace_with` items on the renderer stack.
            let id = currently_rendered
                .find_first_element(dom)
                .expect("suspense placeholders should keep a DOM anchor");
            to.replace_node_with(id, replace_with);
            currently_rendered.remove_node(&mut *dom, Some(to));

            // Switch to only writing templates
            only_write_templates(to);

            children.mount.take();

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create(dom, parent, Some(to));
            });

            set_rendered_children(dom, scope_id, children);

            // Run any closures that were waiting for the suspense to resolve
            suspense_context.run_resolved_closures(&dom.runtime);
        })
    }

    pub(crate) fn diff<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        mut to: Option<&mut M>,
    ) {
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope = &mut dom.scopes[scope_id.0];
            let last_rendered_node = scope.last_rendered_node.clone().unwrap();
            let Self { fallback, children } = Self::downcast_from_props(&mut *scope.props)
                .unwrap()
                .clone();

            let suspense_context = scope.state().suspense_boundary().unwrap().clone();
            let suspended_branch = suspense_context.suspended_branch();
            let suspended = !suspense_context.suspended_futures().is_empty();
            match (suspended_branch, suspended) {
                // We already have suspended nodes that still need to be suspended
                // Just diff the normal and suspended nodes
                (Some(suspended_branch), true) => {
                    let suspended_nodes = suspended_branch.root();
                    let new_suspended_nodes: VNode = children.as_vnode().clone();

                    // Diff the suspended nodes in the background *first*: re-running the
                    // child may cancel its suspend (e.g. a signal flipped a `mode` flag)
                    // and we want to observe that before committing to a fallback render.
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        suspended_nodes.diff_node(&new_suspended_nodes, dom, to.as_deref_mut());
                        while let Some(order) = dom.pop_dirty_descendant_of(scope_id) {
                            dom.run_and_diff_scope(to.as_deref_mut(), order.id);
                        }
                    });

                    if !suspense_context.suspended_futures().is_empty() {
                        // Still suspended: diff the placeholder against a fresh fallback.
                        let new_placeholder =
                            suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                                let new_placeholder =
                                    LastRenderedNode::new(fallback.call(suspense_context.clone()));
                                last_rendered_node.diff_node(&new_placeholder, dom, to);
                                new_placeholder
                            });
                        dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);
                        let branch = SuspenseBranch::new(new_suspended_nodes);
                        store_suspended_branch(dom, scope_id, &branch);
                        suspense_context.set_suspended_branch(branch);
                    } else {
                        // The background diff resolved the suspension. Promote the
                        // background-rendered nodes by replacing the fallback placeholder.
                        suspense_context.take_suspended_branch();
                        dom.set_mount_mode(new_suspended_nodes.mount.get(), RenderMode::Foreground);
                        replace_suspense_nodes(
                            &suspense_context,
                            &last_rendered_node,
                            &new_suspended_nodes,
                            dom,
                            to.as_deref_mut(),
                            |dom| {
                                new_suspended_nodes.remove_node_inner(dom, None::<&mut M>, false);
                            },
                        );
                        set_rendered_children(dom, scope_id, LastRenderedNode::Real(new_suspended_nodes));
                        mark_suspense_resolved(&suspense_context, dom, scope_id);
                    }
                }
                // We have no suspended nodes, and we are not suspended at diff start.
                // Diff the children normally — but a descendant scope may invoke
                // `suspend(task)?` during diff. If that happens we need to
                // retroactively switch this boundary to its fallback so the user
                // never sees a half-rendered tree.
                (None, false) => {
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        last_rendered_node.diff_node(&children, dom, to.as_deref_mut());
                        // diff_node queues descendant component scopes for
                        // diffing instead of running them inline. Flush that
                        // queue here so any `suspend(task)?` they emit lands in
                        // `suspense_context.suspended_futures()` before we
                        // decide whether to commit or fall back.
                        while let Some(order) = dom.pop_dirty_descendant_of(scope_id) {
                            dom.run_and_diff_scope(to.as_deref_mut(), order.id);
                        }
                    });

                    if suspense_context.suspended_futures().is_empty() {
                        set_rendered_children(dom, scope_id, children);
                    } else {
                        // A descendant suspended during diff. Move the just-diffed
                        // children into the background and show the fallback.
                        let new_children: VNode = children.as_vnode().clone();
                        let new_placeholder =
                            LastRenderedNode::new(fallback.call(suspense_context.clone()));

                        let parent = dom.get_mounted_parent(new_children.mount.get());

                        suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                            children.move_node_to_background(
                                std::slice::from_ref(&new_placeholder),
                                parent,
                                dom,
                                to.as_deref_mut(),
                            );
                        });

                        dom.set_mount_mode(new_children.mount.get(), RenderMode::Background);

                        let branch = SuspenseBranch::new(new_children);
                        store_suspended_branch(dom, scope_id, &branch);
                        dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);
                        suspense_context.set_suspended_branch(branch);

                        un_resolve_suspense(dom, scope_id);
                    }
                }
                // We have no suspended nodes, but we just became suspended. Move the children to the background
                (None, true) => {
                    let old_children = last_rendered_node;
                    let new_children: VNode = children.as_vnode().clone();

                    let new_placeholder =
                        LastRenderedNode::new(fallback.call(suspense_context.clone()));

                    // Move the children to the background
                    let parent = dom.get_mounted_parent(old_children.mount.get());

                    suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                        old_children.move_node_to_background(
                            std::slice::from_ref(&new_placeholder),
                            parent,
                            dom,
                            to.as_deref_mut(),
                        );
                    });

                    // Then diff the new children in the background
                    dom.set_mount_mode(old_children.mount.get(), RenderMode::Background);
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        old_children.diff_node(&new_children, dom, to.as_deref_mut());
                    });

                    if suspense_context.suspended_futures().is_empty() {
                        dom.set_mount_mode(new_children.mount.get(), RenderMode::Foreground);
                        replace_suspense_nodes(
                            &suspense_context,
                            &new_placeholder,
                            &new_children,
                            dom,
                            to.as_deref_mut(),
                            |dom| {
                                new_children.remove_node_inner(dom, None::<&mut M>, false);
                            },
                        );

                        set_rendered_children(dom, scope_id, LastRenderedNode::Real(new_children));
                        mark_suspense_resolved(&suspense_context, dom, scope_id);
                    } else {
                        let branch = SuspenseBranch::new(new_children);
                        store_suspended_branch(dom, scope_id, &branch);
                        // Set the last rendered node to the new suspense placeholder
                        dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);
                        suspense_context.set_suspended_branch(branch);

                        un_resolve_suspense(dom, scope_id);
                    }
                }
                // We have suspended nodes, but we just got out of suspense. Move the suspended nodes to the foreground
                (Some(_), false) => {
                    // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
                    let old_suspended_branch = suspense_context.take_suspended_branch().unwrap();
                    dom.set_mount_mode(old_suspended_branch.root_mount(), RenderMode::Foreground);
                    let old_suspended_nodes = old_suspended_branch.into_root();

                    // First diff the two children nodes in the background
                    replace_suspense_nodes(
                        &suspense_context,
                        &last_rendered_node,
                        &children,
                        dom,
                        to,
                        |dom| {
                            old_suspended_nodes.diff_node(&children, dom, None::<&mut M>);
                            promote_resolved_suspense_descendants::<M>(dom, &children);
                        },
                    );

                    set_rendered_children(dom, scope_id, children);

                    mark_suspense_resolved(&suspense_context, dom, scope_id);
                }
            }
        })
    }
}

fn store_suspense_children(dom: &mut VirtualDom, scope_id: ScopeId, children: &LastRenderedNode) {
    let props =
        SuspenseBoundaryProps::downcast_from_props(&mut *dom.scopes[scope_id.0].props).unwrap();
    props.children.clone_from(children);
}

fn set_rendered_children(dom: &mut VirtualDom, scope_id: ScopeId, children: LastRenderedNode) {
    store_suspense_children(dom, scope_id, &children);
    dom.scopes[scope_id.0].last_rendered_node = Some(children);
}

fn store_suspended_branch(dom: &mut VirtualDom, scope_id: ScopeId, branch: &SuspenseBranch) {
    debug_assert!(branch.root_mount().mounted());
    dom.set_mount_mode(branch.root_mount(), RenderMode::Background);
    store_suspense_children(dom, scope_id, &LastRenderedNode::Real(branch.root()));
}

fn replace_suspense_nodes<M: WriteMutations>(
    suspense_context: &SuspenseContext,
    placeholder: &LastRenderedNode,
    children: &VNode,
    dom: &mut VirtualDom,
    to: Option<&mut M>,
    prepare: impl FnOnce(&mut VirtualDom),
) {
    suspense_context.under_suspense_boundary(&dom.runtime(), || {
        prepare(dom);
        let children = dom
            .current_mounted_view(children.mount.get())
            .unwrap_or_else(|| children.clone());
        replace_placeholder_with(placeholder, &children, dom, to);
    });
}

fn replace_placeholder_with<M: WriteMutations>(
    placeholder: &LastRenderedNode,
    children: &VNode,
    dom: &mut VirtualDom,
    mut to: Option<&mut M>,
) {
    let parent = dom.get_mounted_parent(placeholder.mount.get());
    if let Some(to_ref) = to.as_deref_mut() {
        let placeholder_vnode = placeholder.as_vnode();
        if let Some(id) = placeholder_vnode.mounted_root(0, dom) {
            let child_owns_placeholder_id = (0..children.template.roots().len()).any(|root_idx| {
                children
                    .mounted_root(root_idx, dom)
                    .is_some_and(|root_id| root_id == id)
            });

            if !child_owns_placeholder_id {
                let created =
                    dom.create_children(Some(&mut *to_ref), std::slice::from_ref(children), parent);
                to_ref.replace_node_with(id, created);
                placeholder.remove_node_inner(dom, None::<&mut M>, true);
                return;
            }
        }
    }

    placeholder.replace(std::slice::from_ref(children), parent, dom, to);
}

/// Move to a resolved suspense state
fn mark_suspense_resolved(
    suspense_context: &SuspenseContext,
    dom: &mut VirtualDom,
    scope_id: ScopeId,
) {
    dom.resolved_scopes.push(scope_id);
    // Run any closures that were waiting for the suspense to resolve
    suspense_context.run_resolved_closures(&dom.runtime);
}

fn promote_resolved_suspense_descendants<M: WriteMutations>(dom: &mut VirtualDom, vnode: &VNode) {
    let mount = vnode.mount.get();
    if !mount.mounted() {
        return;
    }
    dom.set_mount_mode(mount, RenderMode::Foreground);

    for (idx, dynamic) in vnode.dynamic_nodes.iter().enumerate() {
        match dynamic {
            DynamicNode::Component(_) => {
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                if let Some(height) = dom
                    .runtime
                    .try_get_state(scope_id)
                    .map(|scope| scope.height)
                {
                    let order = ScopeOrder::new(height, scope_id);
                    if dom.dirty_scopes.remove(&order) {
                        let mounted = dom.scopes[scope_id.0]
                            .last_rendered_node
                            .as_ref()
                            .is_some_and(|node| node.mount.get().mounted());
                        if mounted {
                            dom.run_and_diff_scope(None::<&mut M>, scope_id);
                        } else {
                            let new = dom.run_scope(scope_id);
                            dom.scopes[scope_id.0].last_rendered_node =
                                Some(LastRenderedNode::new(new));
                        }
                    }
                }

                if let Some(context) =
                    SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                {
                    SuspenseBoundaryProps::diff::<M>(scope_id, dom, None);
                    if let Some(branch) = context.suspended_branch() {
                        let root = branch.root();
                        dom.set_mount_mode(branch.root_mount(), RenderMode::Foreground);
                        promote_resolved_suspense_descendants::<M>(dom, &root);
                        dom.scopes[scope_id.0].last_rendered_node =
                            Some(LastRenderedNode::Real(root));
                        context.take_suspended_branch();
                    }
                }

                if let Some(rendered) = dom.scopes[scope_id.0].last_rendered_node.clone() {
                    promote_resolved_suspense_descendants::<M>(dom, &rendered);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for node in nodes {
                    promote_resolved_suspense_descendants::<M>(dom, node);
                }
            }
            DynamicNode::Text(_) => {}
        }
    }
}

/// Move from a resolved suspense state to an suspended state
fn un_resolve_suspense(dom: &mut VirtualDom, scope_id: ScopeId) {
    dom.resolved_scopes.retain(|&id| id != scope_id);
}

impl SuspenseContext {
    /// Run a closure under a suspense boundary
    pub(crate) fn under_suspense_boundary<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(
            SuspenseLocation::UnderSuspense {
                boundary: self.clone(),
                hidden_by: inherited_contexts(runtime),
            },
            f,
        )
    }

    /// Run a closure under a suspense placeholder
    pub(crate) fn in_suspense_placeholder<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(
            SuspenseLocation::InSuspensePlaceholder {
                boundary: self.clone(),
                hidden_by: inherited_contexts(runtime),
            },
            f,
        )
    }

    /// Try to get a suspense boundary from a scope id
    pub fn downcast_suspense_boundary_from_scope(
        runtime: &Runtime,
        scope_id: ScopeId,
    ) -> Option<Self> {
        runtime.try_get_state(scope_id)?.suspense_boundary()
    }

    pub(crate) fn remove_suspended_nodes<M: WriteMutations>(
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        destroy_component_state: bool,
    ) {
        if let Some(scope) = Self::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
            && let Some(branch) = scope.take_suspended_branch()
        {
            branch
                .into_root()
                .remove_node_inner(dom, None::<&mut M>, destroy_component_state)
        }
    }
}

fn inherited_contexts(runtime: &Runtime) -> Vec<SuspenseContext> {
    runtime
        .current_suspense_location()
        .map(|l| l.inherited_contexts())
        .unwrap_or_default()
}
