use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    DynamicNode,
    diff::context::DiffContext,
    innerlude::*,
    mount::{RenderMode, SuspenseBranch},
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
        VComponent::new_with_driver(component_name, Rc::new(SuspenseDriver::new(self)))
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
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn SuspenseBoundary(__props: SuspenseBoundaryProps) -> Element {
    unreachable!("SuspenseBoundary should not be called directly")
}

/// The rendering lifecycle of a suspense boundary scope: children render in
/// the background first, and the scope's output is either the children or
/// the fallback depending on whether any descendant suspended. Owns the
/// [`SuspenseBoundaryProps`] it renders from.
struct SuspenseDriver {
    props: RefCell<SuspenseBoundaryPropsWithOwner>,
}

impl SuspenseDriver {
    fn new(props: SuspenseBoundaryPropsWithOwner) -> Self {
        Self {
            props: RefCell::new(props),
        }
    }

    /// The boundary's current children input.
    fn children(&self) -> LastRenderedNode {
        self.props.borrow().inner.children.clone()
    }

    /// The boundary's current fallback input.
    fn fallback(&self) -> Callback<SuspenseContext, Element> {
        self.props.borrow().inner.fallback
    }

    /// Record the children handle the boundary mounted, so later reads (diff
    /// arms, streaming resolution) observe mount-accurate state.
    fn store_children(&self, children: &LastRenderedNode) {
        self.props.borrow_mut().inner.children.clone_from(children);
    }
}

/// The suspense driver owning `scope_id`, which must be a suspense boundary
/// scope. The returned `Rc` keeps the driver borrowable past `&mut dom` uses.
fn suspense_driver(dom: &VirtualDom, scope_id: ScopeId) -> Rc<dyn RenderDriver> {
    dom.runtime.get_state(scope_id).render_driver()
}

fn as_suspense(driver: &Rc<dyn RenderDriver>) -> &SuspenseDriver {
    driver
        .as_any()
        .downcast_ref::<SuspenseDriver>()
        .expect("suspense boundary scopes carry a SuspenseDriver")
}

impl RenderDriver for SuspenseDriver {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn memoize(&self, new_driver: &dyn Any) -> bool {
        match new_driver.downcast_ref::<Self>() {
            Some(new) => Properties::memoize(&mut *self.props.borrow_mut(), &new.props.borrow()),
            None => false,
        }
    }

    fn duplicate(&self) -> Rc<dyn RenderDriver> {
        Rc::new(Self::new(self.props.borrow().clone()))
    }

    fn create(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        new: bool,
        parent: Option<ElementRef>,
        mut to: Option<&mut dyn WriteMutations>,
    ) -> usize {
        if new {
            let suspense_context = SuspenseContext::new();
            let scope_state = dom.runtime.get_state(scope_id);
            scope_state.set_suspense_boundary(suspense_context.clone());
            suspense_context.mount(scope_id);
        }
        suspense_create(self, scope_id, parent, dom, to.as_mut())
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        _parent_context: Option<DiffContext<'_>>,
        mut to: Option<&mut dyn WriteMutations>,
    ) {
        let mut render_to = to.as_mut().filter(|_| dom.scope_should_write_now(scope_id));
        suspense_diff(self, scope_id, dom, render_to.as_mut())
    }

    fn remove(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        to: Option<&mut dyn WriteMutations>,
        destroy_component_state: bool,
    ) {
        // If this is a suspense boundary, remove the suspended nodes as well.
        //
        // When we are only moving a component out of the real DOM for an
        // ancestor suspense boundary, the nested boundary's suspended nodes
        // are still its background state. Keep them so the nested boundary
        // can resume or continue diffing while hidden.
        if destroy_component_state {
            SuspenseContext::remove_suspended_nodes::<&mut dyn WriteMutations>(
                dom,
                scope_id,
                destroy_component_state,
            );
        }

        // The scope's rendered output (children or fallback) is removed the
        // same way a plain component's output is.
        remove_rendered_output(dom, scope_id, to, destroy_component_state);
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
fn suspense_create<M: WriteMutations>(
    driver: &SuspenseDriver,
    scope_id: ScopeId,
    parent: Option<ElementRef>,
    dom: &mut VirtualDom,
    to: Option<&mut M>,
) -> usize {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let suspense_context = dom.runtime.get_state(scope_id).suspense_boundary().unwrap();

        let children = driver.children();

        // First always render the children in the background. Rendering the children may cause this boundary to suspend
        suspense_context.under_suspense_boundary(&dom.runtime(), || {
            children.create(dom, parent, None::<&mut M>);
        });

        driver.store_children(&children);

        // If there are suspended futures, render the fallback

        if !suspense_context.suspended_futures().is_empty() {
            let placeholder_context = suspense_context.clone();
            let (node, nodes_created) =
                suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                    let fallback = driver.fallback();
                    let branch = SuspenseBranch::new(children.as_vnode().clone());
                    store_suspended_branch(driver, dom, &branch);
                    placeholder_context.set_suspended_branch(branch);
                    let suspense_placeholder =
                        LastRenderedNode::new(fallback.call(placeholder_context));
                    let nodes_created = suspense_placeholder.create(dom, parent, to);
                    (suspense_placeholder, nodes_created)
                });

            dom.scopes[scope_id.index()].last_rendered_node = Some(node);
            nodes_created
        } else {
            // Otherwise just render the children in the real dom
            let nodes_created = suspense_context
                .under_suspense_boundary(&dom.runtime(), || children.create(dom, parent, to));
            dom.scopes[scope_id.index()].last_rendered_node = Some(children);
            suspense_context.take_suspended_branch();
            mark_suspense_resolved(&suspense_context, dom, scope_id);

            nodes_created
        }
    })
}

impl SuspenseBoundaryProps {
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
            let Some(scope_state) = dom.scopes.get_mut(scope_id.index()) else {
                return;
            };

            // Reset the suspense context
            let suspense_context = scope_state.state().suspense_boundary().unwrap().clone();
            suspense_context.inner.suspended_tasks.borrow_mut().clear();

            // Get the parent of the suspense boundary to later create children with the right parent
            let currently_rendered = scope_state.last_rendered_node.clone().unwrap();
            let mount = currently_rendered.unchecked_mounted_id();
            let parent = dom.mounted_render_parent(mount);

            let driver = suspense_driver(dom, scope_id);
            let driver = as_suspense(&driver);

            // Unmount any children to reset any scopes under this suspense boundary
            let children = driver.children();
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

            children.clear_mounted_id();

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create(dom, parent, Some(to));
            });

            set_rendered_children(driver, dom, scope_id, children);

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
fn suspense_diff<M: WriteMutations>(
    driver: &SuspenseDriver,
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    mut to: Option<&mut M>,
) {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let scope = &mut dom.scopes[scope_id.index()];
        let last_rendered_node = scope.last_rendered_node.clone().unwrap();
        let children = driver.children();
        let fallback = driver.fallback();

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
                    dom.scopes[scope_id.index()].last_rendered_node = Some(new_placeholder);
                    let branch = SuspenseBranch::new(new_suspended_nodes);
                    store_suspended_branch(driver, dom, &branch);
                    suspense_context.set_suspended_branch(branch);
                } else {
                    // The background diff resolved the suspension. Promote the
                    // background-rendered nodes by replacing the fallback placeholder.
                    suspense_context.take_suspended_branch();
                    promote_resolved_children(
                        driver,
                        &suspense_context,
                        &last_rendered_node,
                        new_suspended_nodes,
                        scope_id,
                        dom,
                        to.as_deref_mut(),
                    );
                }
            }
            // We have no suspended nodes, and we are not suspended. Just diff the children like normal
            (None, false) => {
                suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    last_rendered_node.diff_node(&children, dom, to);
                });

                set_rendered_children(driver, dom, scope_id, children);
            }
            // We have no suspended nodes, but we just became suspended. Move the children to the background
            (None, true) => {
                let old_children = last_rendered_node;
                let new_children: VNode = children.as_vnode().clone();

                let new_placeholder =
                    LastRenderedNode::new(fallback.call(suspense_context.clone()));

                // Move the children to the background
                let parent = dom.get_mounted_parent(old_children.unchecked_mounted_id());

                suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                    old_children.move_node_to_background(
                        std::slice::from_ref(&new_placeholder),
                        parent,
                        dom,
                        to.as_deref_mut(),
                    );
                });

                // Then diff the new children in the background
                dom.set_mount_mode(old_children.unchecked_mounted_id(), RenderMode::Background);
                suspense_context.under_suspense_boundary(&dom.runtime(), || {
                    old_children.diff_node(&new_children, dom, to.as_deref_mut());
                });

                if suspense_context.suspended_futures().is_empty() {
                    promote_resolved_children(
                        driver,
                        &suspense_context,
                        &new_placeholder,
                        new_children,
                        scope_id,
                        dom,
                        to.as_deref_mut(),
                    );
                } else {
                    let branch = SuspenseBranch::new(new_children);
                    store_suspended_branch(driver, dom, &branch);
                    // Set the last rendered node to the new suspense placeholder
                    dom.scopes[scope_id.index()].last_rendered_node = Some(new_placeholder);
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
                        promote_suspense_mounts_to_foreground::<M>(dom, &children);
                    },
                );

                set_rendered_children(driver, dom, scope_id, children);

                mark_suspense_resolved(&suspense_context, dom, scope_id);
            }
        }
    })
}

fn set_rendered_children(
    driver: &SuspenseDriver,
    dom: &mut VirtualDom,
    scope_id: ScopeId,
    children: LastRenderedNode,
) {
    driver.store_children(&children);
    dom.scopes[scope_id.index()].last_rendered_node = Some(children);
}

fn store_suspended_branch(driver: &SuspenseDriver, dom: &mut VirtualDom, branch: &SuspenseBranch) {
    dom.set_mount_mode(branch.root_mount(), RenderMode::Background);
    driver.store_children(&LastRenderedNode::Real(branch.root()));
}

/// Promote freshly-resolved `children` over the visible fallback
/// `placeholder`, record them as the boundary's rendered output, and mark the
/// boundary resolved. Shared by the two diff arms that observe a suspension
/// clearing while a fallback is on screen.
fn promote_resolved_children<M: WriteMutations>(
    driver: &SuspenseDriver,
    suspense_context: &SuspenseContext,
    placeholder: &LastRenderedNode,
    children: VNode,
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    to: Option<&mut M>,
) {
    dom.set_mount_mode(children.unchecked_mounted_id(), RenderMode::Foreground);
    replace_suspense_nodes(suspense_context, placeholder, &children, dom, to, |dom| {
        children.remove_node_inner(dom, None::<&mut M>, false);
    });
    set_rendered_children(driver, dom, scope_id, LastRenderedNode::Real(children));
    mark_suspense_resolved(suspense_context, dom, scope_id);
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
            .current_mounted_view(children.unchecked_mounted_id())
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
    let parent = dom.get_mounted_parent(placeholder.unchecked_mounted_id());
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

fn promote_suspense_mounts_to_foreground<M: WriteMutations>(dom: &mut VirtualDom, vnode: &VNode) {
    let mount = vnode.unchecked_mounted_id();
    dom.set_mount_mode(mount, RenderMode::Foreground);

    for (idx, dynamic) in vnode.dynamic_nodes.iter().enumerate() {
        match dynamic {
            DynamicNode::Component(_) => {
                let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                if dom.mark_clean(scope_id) {
                    dom.run_and_diff_scope(None::<&mut M>, scope_id);
                }

                if let Some(rendered) = dom.scopes[scope_id.index()].last_rendered_node.clone() {
                    promote_suspense_mounts_to_foreground::<M>(dom, &rendered);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for node in nodes {
                    promote_suspense_mounts_to_foreground::<M>(dom, node);
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
