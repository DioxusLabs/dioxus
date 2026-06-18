use std::{any::Any, rc::Rc};

use crate::{
    DynamicNode,
    diff::context::DiffContext,
    innerlude::*,
    mount::{RenderMode, SuspenseBranch},
    mutations::replace_id_with,
    render_driver::{RenderDriver, remove_rendered_output},
    scope_context::SuspenseLocation,
};

/// Properties for the [`SuspenseBoundary()`] component.
#[allow(non_camel_case_types)]
pub struct SuspenseBoundaryProps {
    fallback: Callback<SuspenseContext, Element>,
    /// The children of the suspense boundary
    children: LastRenderedNode,
}

impl Clone for SuspenseBoundaryProps {
    fn clone(&self) -> Self {
        Self {
            fallback: self.fallback,
            children: self.children.clone(),
        }
    }
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
            _phantom: ::core::default::Default::default(),
        }
    }
}
#[must_use]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
/// Builder for [`SuspenseBoundaryProps`].
pub struct SuspenseBoundaryPropsBuilder<TypedBuilderFields> {
    owner: Owner,
    fields: TypedBuilderFields,
    _phantom: (),
}

#[must_use]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
/// Component builder for [`SuspenseBoundary`].
pub struct SuspenseBoundaryComponentBuilder<RenderFn, Marker, TypedBuilderFields> {
    render_fn: RenderFn,
    builder: SuspenseBoundaryPropsBuilder<TypedBuilderFields>,
    _marker: std::marker::PhantomData<fn() -> Marker>,
}

impl Properties for SuspenseBoundaryProps
where
    Self: Clone,
{
    type Builder = SuspenseBoundaryPropsBuilder<((), ())>;
    type ComponentBuilder<RenderFn, Marker> =
        SuspenseBoundaryComponentBuilder<RenderFn, Marker, ((), ())>;

    fn builder() -> Self::Builder {
        SuspenseBoundaryProps::builder()
    }

    fn component_builder<RenderFn, Marker>(
        render_fn: RenderFn,
    ) -> Self::ComponentBuilder<RenderFn, Marker> {
        SuspenseBoundaryComponentBuilder {
            render_fn,
            builder: Self::builder(),
            _marker: std::marker::PhantomData,
        }
    }

    fn memoize(&mut self, new: &Self) -> bool {
        self.fallback.__point_to(&new.fallback);
        self.children = new.children.clone();
        false
    }
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<RenderFn, ComponentMarker, __children>
    SuspenseBoundaryComponentBuilder<RenderFn, ComponentMarker, ((), __children)>
{
    #[allow(clippy::type_complexity)]
    pub fn fallback<__Marker>(
        self,
        fallback: impl SuperInto<Callback<SuspenseContext, Element>, __Marker>,
    ) -> SuspenseBoundaryComponentBuilder<
        RenderFn,
        ComponentMarker,
        ((Callback<SuspenseContext, Element>,), __children),
    > {
        SuspenseBoundaryComponentBuilder {
            render_fn: self.render_fn,
            builder: self.builder.fallback(fallback),
            _marker: self._marker,
        }
    }
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<RenderFn, ComponentMarker, __fallback>
    SuspenseBoundaryComponentBuilder<RenderFn, ComponentMarker, (__fallback, ())>
{
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        children: Element,
    ) -> SuspenseBoundaryComponentBuilder<RenderFn, ComponentMarker, (__fallback, (Element,))> {
        SuspenseBoundaryComponentBuilder {
            render_fn: self.render_fn,
            builder: self.builder.children(children),
            _marker: self._marker,
        }
    }
}
#[allow(dead_code, non_camel_case_types, non_snake_case)]
/// Helper trait used by the generated suspense boundary props builder.
pub trait SuspenseBoundaryPropsBuilder_Optional<T> {
    /// Convert the optional builder field into a value.
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
        let fallback = (with_owner(self.owner.clone(), move || {
            SuperInto::super_into(fallback)
        }),);
        let (_, children) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, children),
            _phantom: self._phantom,
        }
    }
}
#[allow(dead_code, non_camel_case_types, non_snake_case)]
/// Error marker for setting the `fallback` field more than once.
pub enum SuspenseBoundaryPropsBuilder_Error_Repeated_field_fallback {}
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
        let children = (children,);
        let (fallback, _) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, children),
            _phantom: self._phantom,
        }
    }
}
#[allow(dead_code, non_camel_case_types, non_snake_case)]
/// Error marker for setting the `children` field more than once.
pub enum SuspenseBoundaryPropsBuilder_Error_Repeated_field_children {}
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
#[allow(dead_code, non_camel_case_types, non_snake_case)]
/// Error marker for missing the required `fallback` field.
pub enum SuspenseBoundaryPropsBuilder_Error_Missing_required_field_fallback {}
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
#[allow(dead_code, non_camel_case_types, missing_docs)]
/// [`SuspenseBoundaryProps`] bundled with the owner that created its callbacks.
pub struct SuspenseBoundaryPropsWithOwner {
    inner: SuspenseBoundaryProps,
    owner: Owner,
}
#[automatically_derived]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl ::core::clone::Clone for SuspenseBoundaryPropsWithOwner {
    #[inline]
    fn clone(&self) -> SuspenseBoundaryPropsWithOwner {
        SuspenseBoundaryPropsWithOwner {
            inner: ::core::clone::Clone::clone(&self.inner),
            owner: ::core::clone::Clone::clone(&self.owner),
        }
    }
}
impl PartialEq for SuspenseBoundaryPropsWithOwner {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}
impl SuspenseBoundaryPropsWithOwner {
    /// Create a component from the props.
    pub fn into_vcomponent<M: 'static>(
        self,
        render_fn: impl ComponentFunction<SuspenseBoundaryProps, M>,
    ) -> VComponent {
        let component_name = std::any::type_name_of_val(&render_fn);
        let render_fn_ptr = render_fn.fn_ptr();
        let props = Box::new(VProps::new(
            move |wrapper: Self| render_fn.rebuild(wrapper.inner),
            <Self as Properties>::memoize,
            self,
            component_name,
        ));
        VComponent::new_with_driver(
            component_name,
            render_fn_ptr,
            Rc::new(SuspenseDriver),
            props,
        )
    }
}

impl<RenderFn, Marker> ComponentBuilderRender<RenderFn, Marker> for SuspenseBoundaryPropsWithOwner
where
    RenderFn: ComponentFunction<SuspenseBoundaryProps, Marker>,
    Marker: 'static,
{
    fn into_vcomponent(self, render_fn: RenderFn) -> VComponent {
        SuspenseBoundaryPropsWithOwner::into_vcomponent(self, render_fn)
    }
}

impl Properties for SuspenseBoundaryPropsWithOwner {
    type Builder = ();
    type ComponentBuilder<RenderFn, Marker> = ();

    fn builder() -> Self::Builder {
        unreachable!()
    }

    fn component_builder<RenderFn, Marker>(
        _render_fn: RenderFn,
    ) -> Self::ComponentBuilder<RenderFn, Marker> {
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
        let fallback = fallback.0;
        let children = SuspenseBoundaryPropsBuilder_Optional::into_value(children, VNode::empty);
        SuspenseBoundaryPropsWithOwner {
            inner: SuspenseBoundaryProps {
                fallback,
                children: LastRenderedNode::new(children),
            },
            owner: self.owner,
        }
    }
}

#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<RenderFn, ComponentMarker, __children: SuspenseBoundaryPropsBuilder_Optional<Element>>
    SuspenseBoundaryComponentBuilder<
        RenderFn,
        ComponentMarker,
        ((Callback<SuspenseContext, Element>,), __children),
    >
{
    pub fn build(
        self,
    ) -> ComponentBuilderOutput<RenderFn, SuspenseBoundaryPropsWithOwner, ComponentMarker> {
        ComponentBuilderOutput::new(self.render_fn, self.builder.build())
    }
}

#[automatically_derived]
#[allow(non_camel_case_types)]
impl ::core::cmp::PartialEq for SuspenseBoundaryProps {
    #[inline]
    fn eq(&self, other: &SuspenseBoundaryProps) -> bool {
        self.fallback == other.fallback && self.children == other.children
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
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn SuspenseBoundary(__props: SuspenseBoundaryProps) -> Element {
    unreachable!("SuspenseBoundary should not be called directly")
}

/// The rendering lifecycle of a suspense boundary scope.
struct SuspenseDriver;

fn suspense_props(dom: &VirtualDom, scope_id: ScopeId) -> &SuspenseBoundaryPropsWithOwner {
    dom.scopes[scope_id.index()]
        .props
        .props()
        .downcast_ref::<SuspenseBoundaryPropsWithOwner>()
        .expect("suspense boundary scope carries SuspenseBoundaryPropsWithOwner")
}

fn suspense_children(dom: &VirtualDom, scope_id: ScopeId) -> LastRenderedNode {
    suspense_props(dom, scope_id).inner.children.clone()
}

fn suspense_fallback(dom: &VirtualDom, scope_id: ScopeId) -> Callback<SuspenseContext, Element> {
    suspense_props(dom, scope_id).inner.fallback
}

fn store_suspense_children(dom: &mut VirtualDom, scope_id: ScopeId, children: &LastRenderedNode) {
    let props = dom.scopes[scope_id.index()]
        .props
        .props_mut()
        .downcast_mut::<SuspenseBoundaryPropsWithOwner>()
        .expect("suspense boundary scope carries SuspenseBoundaryPropsWithOwner");
    props.inner.children.clone_from(children);
}

impl RenderDriver for SuspenseDriver {
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
        if !new {
            let rendered = dom.scopes[scope_id.index()]
                .last_rendered_node
                .as_ref()
                .expect("suspense")
                .node()
                .clone();
            dom.mark_clean(scope_id);
            return dom.create_scope(to, scope_id, rendered, parent);
        }

        if new {
            let suspense_context = SuspenseContext::new();
            let scope_state = dom.runtime.get_state(scope_id);
            scope_state.set_suspense_boundary(suspense_context.clone());
            suspense_context.mount(scope_id);
        }
        let nodes = suspense_create(scope_id, parent, dom, to);
        dom.mark_clean(scope_id);
        nodes
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        _parent_context: Option<DiffContext<'_>>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) {
        let render_to = to.filter(|_| dom.scope_should_write_now(scope_id));
        suspense_diff(scope_id, dom, render_to)
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
    ) {
        // If this is a suspense boundary, remove the suspended nodes as well.
        //
        // When we are only moving a component out of the real DOM for an
        // ancestor suspense boundary, the nested boundary's suspended nodes
        // are still its background state. Keep them so the nested boundary
        // can resume or continue diffing while hidden.
        if destroy_component_state {
            SuspenseContext::remove_suspended_nodes(dom, scope_id, destroy_component_state);
        }

        // The scope's rendered output (children or fallback) is removed the
        // same way a plain component's output is.
        remove_rendered_output(dom, scope_id, to, destroy_component_state);
    }
}
#[allow(non_snake_case)]
mod SuspenseBoundary_completions {
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        /// Autocomplete variant for `SuspenseBoundary`.
        SuspenseBoundary {},
    }
}
#[allow(unused)]
pub use SuspenseBoundary_completions::Component::SuspenseBoundary;
use generational_box::Owner;

/// Mount a suspense boundary scope: render the children in the background
/// first, then mount either the children or the fallback depending on whether
/// anything suspended.
fn suspense_create(
    scope_id: ScopeId,
    parent: Option<MountRef>,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) -> usize {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let suspense_context = dom.runtime.get_state(scope_id).suspense_boundary().unwrap();

        let children = suspense_children(dom, scope_id);

        // First always render the children in the background. Rendering the children may cause this boundary to suspend
        let background = suspense_context.under_suspense_boundary(&dom.runtime(), || {
            children.create_with_parents(dom, parent, parent, None)
        });

        store_suspense_children(dom, scope_id, &children);

        // If there are suspended futures, render the fallback

        if !suspense_context.suspended_futures().is_empty() {
            let placeholder_context = suspense_context.clone();
            let (node, nodes_created) =
                suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                    let fallback = suspense_fallback(dom, scope_id);
                    let branch = SuspenseBranch::new(children.as_vnode().clone(), background.mount);
                    store_suspended_branch(dom, scope_id, &branch);
                    placeholder_context.set_suspended_branch(branch);
                    let suspense_placeholder =
                        LastRenderedNode::new(fallback.call(placeholder_context));
                    let nodes_created =
                        suspense_placeholder.create_with_parents(dom, parent, parent, to);
                    (suspense_placeholder, nodes_created)
                });

            dom.scopes[scope_id.index()].last_rendered_node =
                Some(MountedOutput::new(node, nodes_created.mount));
            nodes_created.nodes
        } else {
            // Otherwise just render the children in the real dom
            let nodes_created = suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children
                    .as_vnode()
                    .recreate_with_mount(dom, background.mount, parent, parent, to)
            });
            dom.scopes[scope_id.index()].last_rendered_node =
                Some(MountedOutput::new(children, nodes_created.mount));
            suspense_context.take_suspended_branch();
            mark_suspense_resolved(&suspense_context, dom, scope_id);

            nodes_created.nodes
        }
    })
}

impl SuspenseBoundaryProps {
    /// Manually rerun the children of this suspense boundary without diffing against the old nodes.
    ///
    /// This should only be called by dioxus-web after the suspense boundary has been streamed in from the server.
    pub fn resolve_suspense<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        to: &mut M,
        push_replacements: impl FnOnce(&mut M) -> usize,
    ) {
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let _runtime = RuntimeGuard::new(dom.runtime());
            let Some(scope_state) = dom.scopes.get_mut(scope_id.index()) else {
                return;
            };

            // Reset the suspense context
            let suspense_context = scope_state.state().suspense_boundary().unwrap().clone();
            suspense_context.inner.suspended_tasks.borrow_mut().clear();

            // Get the parent of the suspense boundary to later create children with the right parent
            let currently_rendered = scope_state.last_rendered_node.clone().unwrap();
            let mount = currently_rendered.root_mount();
            let parent = dom.mounted_render_parent(mount);

            // Unmount any children to reset any scopes under this suspense boundary
            let children = suspense_children(dom, scope_id);
            // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
            if let Some(branch) = suspense_context.take_suspended_branch() {
                let mount = branch.root_mount();
                branch.into_root().remove_node(mount, &mut *dom, None);
            }

            // Streaming replacements are pushed after the placeholder target
            // so `replace_with` can stay stack-only.
            let id = currently_rendered
                .mounted_vnode()
                .find_first_element(dom)
                .expect("placeholder");
            replace_id_with(to, id, push_replacements);
            currently_rendered.as_vnode().remove_node(
                currently_rendered.root_mount(),
                &mut *dom,
                None,
            );

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            let created = suspense_context.under_suspense_boundary(&dom.runtime(), || {
                let mut no_op = crate::NoOpMutations;
                children.create_with_parents(dom, parent, parent, Some(&mut no_op))
            });

            set_rendered_children(dom, scope_id, children, created.mount);

            // Run any closures that were waiting for the suspense to resolve
            suspense_context.run_resolved_closures(&dom.runtime);
        })
    }
}

/// Diff a suspense boundary scope against its current children/fallback
/// props.
///
/// `to` is the pre-gated writer: [`SuspenseDriver::diff`] applies the scope
/// write gate before calling in.
fn suspense_diff(
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    mut to: Option<&mut (dyn WriteMutations + '_)>,
) {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let scope = &dom.scopes[scope_id.index()];
        let last_rendered_node = scope.last_rendered_node.clone().unwrap();
        let children = suspense_children(dom, scope_id);
        let fallback = suspense_fallback(dom, scope_id);

        let suspense_context = scope.state().suspense_boundary().unwrap().clone();
        let suspended_branch = suspense_context.suspended_branch();
        let suspended = !suspense_context.suspended_futures().is_empty();
        match (suspended_branch, suspended) {
            // We already have suspended nodes that still need to be suspended
            // Just diff the normal and suspended nodes
            (Some(suspended_branch), true) => {
                let suspended_nodes = suspended_branch.root();
                let suspended_mount = suspended_branch.root_mount();
                let new_suspended_nodes: VNode = children.as_vnode().clone();

                // Diff the suspended nodes in the background *first*: re-running the
                // child may cancel its suspend (e.g. a signal flipped a `mode` flag)
                // and we want to observe that before committing to a fallback render.
                let new_suspended_mount =
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        MountedVNode::new(&suspended_nodes, suspended_mount).diff_node(
                            &new_suspended_nodes,
                            dom,
                            to.as_deref_mut(),
                        )
                    });
                flush_retained_branch_scopes(dom, scope_id);

                if !suspense_context.suspended_futures().is_empty() {
                    // Still suspended: diff the placeholder against a fresh fallback.
                    let (new_placeholder, placeholder_mount) = suspense_context
                        .in_suspense_placeholder(&dom.runtime(), || {
                            let new_placeholder =
                                LastRenderedNode::new(fallback.call(suspense_context.clone()));
                            let placeholder_mount = last_rendered_node.mounted_vnode().diff_node(
                                new_placeholder.as_vnode(),
                                dom,
                                to,
                            );
                            (new_placeholder, placeholder_mount)
                        });
                    dom.scopes[scope_id.index()].last_rendered_node =
                        Some(MountedOutput::new(new_placeholder, placeholder_mount));
                    let branch = SuspenseBranch::new(new_suspended_nodes, new_suspended_mount);
                    store_suspended_branch(dom, scope_id, &branch);
                    suspense_context.set_suspended_branch(branch);
                } else {
                    // The background diff resolved the suspension. Promote the
                    // background-rendered nodes by replacing the fallback placeholder.
                    suspense_context.take_suspended_branch();
                    promote_resolved_children(
                        &suspense_context,
                        &last_rendered_node,
                        new_suspended_nodes,
                        new_suspended_mount,
                        scope_id,
                        dom,
                        to.as_deref_mut(),
                    );
                }
            }
            // We have no suspended nodes, and we are not suspended. Just diff the children like normal
            (None, false) => {
                let new_mount = suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    last_rendered_node
                        .mounted_vnode()
                        .diff_node(children.as_vnode(), dom, to)
                });

                set_rendered_children(dom, scope_id, children, new_mount);
            }
            // We have no suspended nodes, but we just became suspended. Move the children to the background
            (None, true) => {
                let old_children = last_rendered_node;
                let new_children: VNode = children.as_vnode().clone();

                let new_placeholder =
                    LastRenderedNode::new(fallback.call(suspense_context.clone()));

                // Move the children to the background
                let old_children_mount = old_children.root_mount();
                let parent = dom.mounted_render_parent(old_children_mount);

                let placeholder_mount =
                    suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                        let created = old_children.as_vnode().move_node_to_background(
                            old_children_mount,
                            std::slice::from_ref(new_placeholder.as_vnode()),
                            parent,
                            dom,
                            to.as_deref_mut(),
                        );
                        created.mounts[0]
                    });

                // Then diff the new children in the background
                set_suspense_mounts_render_mode(
                    dom,
                    old_children.as_vnode(),
                    old_children_mount,
                    RenderMode::Background,
                );
                let new_children_mount =
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        old_children.mounted_vnode().diff_node(
                            &new_children,
                            dom,
                            to.as_deref_mut(),
                        )
                    });
                flush_retained_branch_scopes(dom, scope_id);

                if suspense_context.suspended_futures().is_empty() {
                    let placeholder_output = MountedOutput::new(new_placeholder, placeholder_mount);
                    promote_resolved_children(
                        &suspense_context,
                        &placeholder_output,
                        new_children,
                        new_children_mount,
                        scope_id,
                        dom,
                        to.as_deref_mut(),
                    );
                } else {
                    let branch = SuspenseBranch::new(new_children, new_children_mount);
                    store_suspended_branch(dom, scope_id, &branch);
                    // Set the last rendered node to the new suspense placeholder
                    dom.scopes[scope_id.index()].last_rendered_node =
                        Some(MountedOutput::new(new_placeholder, placeholder_mount));
                    suspense_context.set_suspended_branch(branch);

                    un_resolve_suspense(dom, scope_id);
                }
            }
            // We have suspended nodes, but we just got out of suspense. Move the suspended nodes to the foreground
            (Some(_), false) => {
                // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
                let old_suspended_branch = suspense_context.take_suspended_branch().unwrap();
                let old_suspended_mount = old_suspended_branch.root_mount();
                dom.set_mount_mode(old_suspended_mount, RenderMode::Foreground);
                let old_suspended_nodes = old_suspended_branch.root();

                // First diff the two children nodes in the background
                let mut new_children_mount = old_suspended_mount;
                replace_suspense_nodes(
                    &suspense_context,
                    &last_rendered_node,
                    old_suspended_mount,
                    dom,
                    to,
                    |dom| {
                        new_children_mount = MountedVNode::new(
                            &old_suspended_nodes,
                            old_suspended_mount,
                        )
                        .diff_node(children.as_vnode(), dom, None);
                        promote_suspense_mounts_to_foreground(
                            dom,
                            children.as_vnode(),
                            new_children_mount,
                        );
                    },
                );

                set_rendered_children(dom, scope_id, children, new_children_mount);

                mark_suspense_resolved(&suspense_context, dom, scope_id);
            }
        }
    })
}

fn flush_retained_branch_scopes(dom: &mut VirtualDom, scope_id: ScopeId) {
    while let Some(order) = dom.pop_dirty_descendant_scope(scope_id) {
        let dirty_scope = order.id;
        let run_scope = dom
            .runtime
            .try_get_state(dirty_scope)
            .filter(|scope| scope.should_run_during_suspense())
            .is_some();
        if run_scope {
            dom.runtime.clone().while_rendering(|| {
                dom.run_and_diff_scope_with_context(None, dirty_scope, None);
            });
        }
    }
}

fn set_rendered_children(
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    children: LastRenderedNode,
    root_mount: MountId,
) {
    store_suspense_children(dom, scope_id, &children);
    dom.scopes[scope_id.index()].last_rendered_node =
        Some(MountedOutput::new(children, root_mount));
}

fn store_suspended_branch(dom: &mut VirtualDom, scope_id: ScopeId, branch: &SuspenseBranch) {
    set_suspense_mounts_render_mode(
        dom,
        &branch.root(),
        branch.root_mount(),
        RenderMode::Background,
    );
    store_suspense_children(dom, scope_id, &LastRenderedNode::Real(branch.root()));
}

/// Promote freshly-resolved `children` over the visible fallback
/// `placeholder`, record them as the boundary's rendered output, and mark the
/// boundary resolved. Shared by the two diff arms that observe a suspension
/// clearing while a fallback is on screen.
fn promote_resolved_children(
    suspense_context: &SuspenseContext,
    placeholder: &MountedOutput,
    children: VNode,
    children_mount: MountId,
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) {
    set_suspense_mounts_render_mode(dom, &children, children_mount, RenderMode::Foreground);
    replace_suspense_nodes(
        suspense_context,
        placeholder,
        children_mount,
        dom,
        to,
        |dom| {
            children.remove_node_inner(children_mount, dom, None, false);
        },
    );
    set_rendered_children(
        dom,
        scope_id,
        LastRenderedNode::Real(children),
        children_mount,
    );
    mark_suspense_resolved(suspense_context, dom, scope_id);
}

fn replace_suspense_nodes(
    suspense_context: &SuspenseContext,
    placeholder: &MountedOutput,
    children_mount: MountId,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
    prepare: impl FnOnce(&mut VirtualDom),
) {
    // Invariant: `children_mount` is the retained branch mount being promoted. The mount must keep
    // its committed vnode through `prepare`; promotion moves ownership foreground instead of
    // reconstructing from props.
    suspense_context.under_suspense_boundary(&dom.runtime(), || {
        prepare(dom);
        let children = dom
            .current_mounted_view(children_mount)
            .expect("suspense child");
        replace_placeholder_with(
            placeholder,
            MountedVNode::new(&children, children_mount),
            dom,
            to,
        );
    });
}

fn replace_placeholder_with(
    placeholder: &MountedOutput,
    children: MountedVNode<'_>,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) {
    // Invariant: `placeholder` is the currently visible branch and `children` is an already
    // materialized retained branch. Replacement may be a no-op when both branches already share the
    // same root id after streaming/promote bookkeeping.
    let parent = dom.mounted_render_parent(placeholder.root_mount());
    if to.is_some() {
        if let Some(id) = placeholder.mounted_vnode().mounted_root(0, dom) {
            let child_owns_placeholder_id = (0..children.template.root_count()).any(|root_idx| {
                children
                    .mounted_root(root_idx, dom)
                    .is_some_and(|root_id| root_id == id)
            });

            if child_owns_placeholder_id {
                return;
            }
        }
    }

    placeholder.as_vnode().replace_with_existing_mount(
        placeholder.root_mount(),
        children.vnode(),
        children.mount(),
        parent,
        dom,
        to,
    );
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

fn promote_suspense_mounts_to_foreground(dom: &mut VirtualDom, vnode: &VNode, mount: MountId) {
    // Invariant: `vnode` is the committed view for `mount`; recursively promoting a retained branch
    // must visit every mounted descendant before renderer-visible writes resume.
    set_suspense_mounts_render_mode(dom, vnode, mount, RenderMode::Foreground);

    for anchor in vnode.template.anchors() {
        for idx in vnode.dynamic_node_indices_for_anchor(anchor) {
            match vnode.dynamic_values[idx].node() {
                DynamicNode::Component(_) => {
                    let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                    if dom.mark_clean(scope_id) {
                        dom.run_and_diff_scope_with_context(None, scope_id, None);
                    }

                    let rendered = dom.scopes[scope_id.index()]
                        .last_rendered_node
                        .clone()
                        .expect("scope output");
                    promote_suspense_mounts_to_foreground(
                        dom,
                        rendered.as_vnode(),
                        rendered.root_mount(),
                    );
                }
                DynamicNode::Fragment(nodes) => {
                    let mounts = dom.mounted_fragment_children_exact(mount, idx, nodes.len());
                    for (node, mount) in nodes.iter().zip(mounts) {
                        promote_suspense_mounts_to_foreground(dom, node, mount);
                    }
                }
                DynamicNode::Text(_) => {}
            }
        }
    }
}

fn set_suspense_mounts_render_mode(
    dom: &mut VirtualDom,
    vnode: &VNode,
    mount: MountId,
    mode: RenderMode,
) {
    // Invariant: the retained suspense branch mode is subtree-wide. A foreground
    // bit inside a hidden branch may emit renderer writes under an ancestor that
    // intentionally owns no live roots, so every mounted descendant follows the
    // branch root before placement can run again.
    dom.set_mount_mode(mount, mode);

    for anchor in vnode.template.anchors() {
        for idx in vnode.dynamic_node_indices_for_anchor(anchor) {
            match vnode.dynamic_values[idx].node() {
                DynamicNode::Component(_) => {
                    let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                    let rendered = dom.scopes[scope_id.index()]
                        .last_rendered_node
                        .clone()
                        .expect("scope output");
                    set_suspense_mounts_render_mode(
                        dom,
                        rendered.as_vnode(),
                        rendered.root_mount(),
                        mode,
                    );
                }
                DynamicNode::Fragment(nodes) => {
                    let mounts = dom.mounted_fragment_children_exact(mount, idx, nodes.len());
                    for (node, mount) in nodes.iter().zip(mounts) {
                        set_suspense_mounts_render_mode(dom, node, mount, mode);
                    }
                }
                DynamicNode::Text(_) => {}
            }
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

    pub(crate) fn remove_suspended_nodes(
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        destroy_component_state: bool,
    ) {
        if let Some(scope) = Self::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
            && let Some(branch) = scope.take_suspended_branch()
        {
            let mount = branch.root_mount();
            branch
                .into_root()
                .remove_node_inner(mount, dom, None, destroy_component_state)
        }
    }
}

fn inherited_contexts(runtime: &Runtime) -> Vec<SuspenseContext> {
    runtime
        .current_suspense_location()
        .map(|l| l.inherited_contexts())
        .unwrap_or_default()
}
