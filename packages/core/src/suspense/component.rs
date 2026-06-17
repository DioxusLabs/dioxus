use crate::{innerlude::*, render_driver::RenderDriver, scope_context::SuspenseLocation};

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
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct SuspenseBoundaryPropsBuilder<TypedBuilderFields> {
    owner: Owner,
    fields: TypedBuilderFields,
    _phantom: (),
}
impl Properties for SuspenseBoundaryProps
where
    Self: Clone,
{
    type Builder = SuspenseBoundaryPropsBuilder<((), ())>;
    fn builder() -> Self::Builder {
        SuspenseBoundaryProps::builder()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        let equal = self == new;
        self.fallback.__point_to(&new.fallback);
        if !equal {
            let new_clone = new.clone();
            self.children = new_clone.children;
        }
        equal
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
        let children = (children,);
        let (fallback, _) = self.fields;
        SuspenseBoundaryPropsBuilder {
            owner: self.owner,
            fields: (fallback, children),
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
#[allow(dead_code, non_camel_case_types, missing_docs)]
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
        let props = VProps::new(
            move |wrapper: Self| render_fn.rebuild(wrapper.inner),
            <Self as Properties>::memoize,
            self,
            component_name,
        );
        VComponent::new_with_driver(
            component_name,
            render_fn_ptr,
            SuspenseDriver::new(),
            Box::new(props),
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
pub fn SuspenseBoundary(__props: SuspenseBoundaryProps) -> Element {
    unreachable!("SuspenseBoundary should not be called directly")
}

/// The rendering lifecycle of a suspense boundary scope.
///
/// The driver owns the [`SuspenseContext`] for this boundary. Children render in the background first; the
/// scope's visible output is either the children or the fallback depending on
/// whether any descendant is suspended.
pub(crate) struct SuspenseDriver {
    /// The suspense context owned by this boundary.
    suspense_context: SuspenseContext,
}

impl SuspenseDriver {
    fn new() -> Self {
        Self {
            suspense_context: SuspenseContext::new(),
        }
    }

    /// Get the suspense context for this boundary.
    pub(crate) fn context(&self) -> SuspenseContext {
        self.suspense_context.clone()
    }
}

impl RenderDriver for SuspenseDriver {
    fn initial_suspense_location(&self, _parent: SuspenseLocation) -> SuspenseLocation {
        SuspenseLocation::SuspenseBoundary(self.suspense_context.clone())
    }

    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) -> usize {
        if new {
            self.suspense_context.mount(scope_id);
        }
        suspense_create(self, scope_id, parent, dom, to)
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
    ) {
        suspense_diff(self, scope_id, dom, to)
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut (dyn WriteMutations + '_)>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        // If this is a suspense boundary, remove the suspended nodes as well
        SuspenseContext::remove_suspended_nodes(dom, scope_id, destroy_component_state);

        if let Some(node) = dom.scopes[scope_id.0].last_rendered_node.clone() {
            node.remove_node_inner(dom, to, destroy_component_state, replace_with)
        };

        if destroy_component_state {
            dom.drop_scope(scope_id);
        }
    }
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

/// Mount a suspense boundary scope: render the children in the background
/// first, then mount either the children or the fallback depending on whether
/// anything suspended.
fn suspense_create(
    driver: &SuspenseDriver,
    scope_id: ScopeId,
    parent: Option<ElementRef>,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) -> usize {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let suspense_context = driver.context();

        let children = suspense_children(scope_id, dom);

        // First always render the children in the background. Rendering the children may cause this boundary to suspend
        suspense_context.under_suspense_boundary(&dom.runtime(), || {
            children.create(dom, parent, None);
        });

        // Store the (now mounted) children back
        store_suspense_children(scope_id, dom, &children);

        // If there are suspended futures, render the fallback
        if !suspense_context.suspended_futures().is_empty() {
            let (node, nodes_created) =
                suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                    suspense_context.set_suspended_nodes(children.as_vnode().clone());
                    let suspense_placeholder = LastRenderedNode::new(
                        suspense_fallback(scope_id, dom).call(suspense_context.clone()),
                    );
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
            dom.scopes[scope_id.0].last_rendered_node = children.into();
            suspense_context.take_suspended_nodes();
            mark_suspense_resolved(&suspense_context, dom, scope_id);

            nodes_created
        }
    })
}

fn suspense_props(scope_id: ScopeId, dom: &mut VirtualDom) -> &mut SuspenseBoundaryProps {
    SuspenseBoundaryProps::downcast_from_props(dom.scopes[scope_id.0].props.as_mut())
        .expect("expected suspense props on suspense boundary scope")
}

fn suspense_children(scope_id: ScopeId, dom: &mut VirtualDom) -> LastRenderedNode {
    suspense_props(scope_id, dom).children.clone()
}

fn suspense_fallback(
    scope_id: ScopeId,
    dom: &mut VirtualDom,
) -> Callback<SuspenseContext, Element> {
    suspense_props(scope_id, dom).fallback
}

fn store_suspense_children(scope_id: ScopeId, dom: &mut VirtualDom, children: &LastRenderedNode) {
    suspense_props(scope_id, dom).children.clone_from(children);
}

impl SuspenseBoundaryProps {
    /// Try to downcast [`AnyProps`] to [`SuspenseBoundaryProps`].
    pub(crate) fn downcast_from_props(props: &mut dyn AnyProps) -> Option<&mut Self> {
        let inner: Option<&mut SuspenseBoundaryPropsWithOwner> = props.props_mut().downcast_mut();
        inner.map(|inner| &mut inner.inner)
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
            let (currently_rendered, parent) = {
                let Some(scope_state) = dom.scopes.get_mut(scope_id.0) else {
                    return;
                };

                // Reset the suspense context
                let suspense_context = scope_state.state().suspense_boundary().unwrap().clone();
                suspense_context.inner.suspended_tasks.borrow_mut().clear();

                // Get the parent of the suspense boundary to later create children with the right parent
                let currently_rendered = scope_state.last_rendered_node.clone().unwrap();
                let mount = currently_rendered.mount.get();
                let parent = {
                    let mounts = dom.runtime.mounts.borrow();
                    mounts
                        .get(mount.0)
                        .expect("suspense placeholder is not mounted")
                        .parent
                };

                (currently_rendered, parent)
            };

            // Unmount any children to reset any scopes under this suspense boundary
            let children = suspense_children(scope_id, dom);
            let suspense_context =
                SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                    .unwrap();

            // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
            let suspended = suspense_context.take_suspended_nodes();
            if let Some(node) = suspended {
                node.remove_node(&mut *dom, None, None);
            }

            // Replace the rendered nodes with resolved nodes
            currently_rendered.remove_node(&mut *dom, Some(&mut *to), Some(replace_with));

            // Switch to only writing templates
            only_write_templates(to);

            children.mount.take();

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create(dom, parent, Some(&mut *to));
            });

            // Store the (now mounted) children back
            store_suspense_children(scope_id, dom, &children);
            dom.scopes[scope_id.0].last_rendered_node = Some(children);

            // Run any closures that were waiting for the suspense to resolve
            suspense_context.run_resolved_closures(&dom.runtime);
        })
    }
}

/// Diff a suspense boundary scope against its current children/fallback props.
fn suspense_diff(
    _driver: &SuspenseDriver,
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    to: Option<&mut (dyn WriteMutations + '_)>,
) {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let last_rendered_node = dom.scopes[scope_id.0].last_rendered_node.clone().unwrap();

        let children = suspense_children(scope_id, dom);
        let fallback = suspense_fallback(scope_id, dom);

        let suspense_context = dom.scopes[scope_id.0]
            .state()
            .suspense_boundary()
            .unwrap()
            .clone();
        let suspended_nodes = suspense_context.suspended_nodes();
        let suspended = !suspense_context.suspended_futures().is_empty();
        match (suspended_nodes, suspended) {
            // We already have suspended nodes that still need to be suspended
            // Just diff the normal and suspended nodes
            (Some(suspended_nodes), true) => {
                let new_suspended_nodes: VNode = children.as_vnode().clone();

                // Diff the placeholder nodes in the dom
                let new_placeholder =
                    suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                        let new_placeholder =
                            LastRenderedNode::new(fallback.call(suspense_context.clone()));

                        last_rendered_node.diff_node(&new_placeholder, dom, to);
                        new_placeholder
                    });

                // Set the last rendered node to the placeholder
                dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);

                // Diff the suspended nodes in the background
                suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    suspended_nodes.diff_node(&new_suspended_nodes, dom, None);
                });

                let suspense_context =
                    SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                        .unwrap();
                suspense_context.set_suspended_nodes(new_suspended_nodes);

                store_suspense_children(scope_id, dom, &children);
            }
            // We have no suspended nodes, and we are not suspended. Just diff the children like normal
            (None, false) => {
                suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    last_rendered_node.diff_node(&children, dom, to);
                });

                // Set the last rendered node to the new children
                store_suspense_children(scope_id, dom, &children);
                dom.scopes[scope_id.0].last_rendered_node = Some(children);
            }
            // We have no suspended nodes, but we just became suspended. Move the children to the background
            (None, true) => {
                let old_children = last_rendered_node;
                let new_children: VNode = children.as_vnode().clone();

                let new_placeholder =
                    LastRenderedNode::new(fallback.call(suspense_context.clone()));

                // Move the children to the background
                let mount = old_children.mount.get();
                let parent = dom.get_mounted_parent(mount);

                suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                    old_children.move_node_to_background(
                        std::slice::from_ref(&new_placeholder),
                        parent,
                        dom,
                        to,
                    );
                });

                // Then diff the new children in the background
                suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    old_children.diff_node(&new_children, dom, None);
                });

                // Set the last rendered node to the new suspense placeholder
                dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);

                let suspense_context =
                    SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                        .unwrap();
                suspense_context.set_suspended_nodes(new_children);

                store_suspense_children(scope_id, dom, &children);
                un_resolve_suspense(dom, scope_id);
            }
            // We have suspended nodes, but we just got out of suspense. Move the suspended nodes to the foreground
            (Some(_), false) => {
                // Take the suspended nodes out of the suspense boundary so the children know that the boundary is not suspended while diffing
                let old_suspended_nodes = suspense_context.take_suspended_nodes().unwrap();
                let old_placeholder = last_rendered_node;

                // First diff the two children nodes in the background
                suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    old_suspended_nodes.diff_node(&children, dom, None);

                    // Then replace the placeholder with the new children
                    let mount = old_placeholder.mount.get();
                    let parent = dom.get_mounted_parent(mount);
                    old_placeholder.replace(std::slice::from_ref(&children), parent, dom, to);
                });

                // Set the last rendered node to the new children
                store_suspense_children(scope_id, dom, &children);
                dom.scopes[scope_id.0].last_rendered_node = Some(children);

                mark_suspense_resolved(&suspense_context, dom, scope_id);
            }
        }
    })
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

/// Move from a resolved suspense state to an suspended state
fn un_resolve_suspense(dom: &mut VirtualDom, scope_id: ScopeId) {
    dom.resolved_scopes.retain(|&id| id != scope_id);
}

impl SuspenseContext {
    /// Run a closure under a suspense boundary
    pub(crate) fn under_suspense_boundary<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(SuspenseLocation::UnderSuspense(self.clone()), f)
    }

    /// Run a closure under a suspense placeholder
    pub(crate) fn in_suspense_placeholder<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(SuspenseLocation::InSuspensePlaceholder(self.clone()), f)
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
        let Some(scope) = Self::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
        else {
            return;
        };
        // Remove the suspended nodes
        if let Some(node) = scope.take_suspended_nodes() {
            node.remove_node_inner(dom, None, destroy_component_state, None)
        }
    }
}
