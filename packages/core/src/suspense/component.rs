use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    DynamicNode,
    diff::context::DiffContext,
    innerlude::*,
    mount::{RenderMode, SuspenseBranch},
    mutations::{reborrow_writer, replace_id_with},
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
        parent: Option<MountRef>,
        mut to: Option<&mut dyn WriteMutations>,
    ) -> usize {
        if new {
            let suspense_context = SuspenseContext::new();
            let scope_state = dom.runtime.get_state(scope_id);
            scope_state.set_suspense_boundary(suspense_context.clone());
            suspense_context.mount(scope_id);
        }
        suspense_create(self, scope_id, parent, dom, reborrow_writer(&mut to))
    }

    fn diff(
        &self,
        dom: &mut VirtualDom,
        scope_id: ScopeId,
        _parent_context: Option<DiffContext<'_>>,
        to: Option<&mut dyn WriteMutations>,
    ) {
        let render_to = to.filter(|_| dom.scope_should_write_now(scope_id));
        suspense_diff(self, scope_id, dom, render_to)
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
            SuspenseContext::remove_suspended_nodes(dom, scope_id, destroy_component_state);
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
fn suspense_create(
    driver: &SuspenseDriver,
    scope_id: ScopeId,
    parent: Option<MountRef>,
    dom: &mut VirtualDom,
    to: Option<&mut dyn WriteMutations>,
) -> usize {
    dom.runtime.clone().with_scope_on_stack(scope_id, || {
        let suspense_context = dom.runtime.get_state(scope_id).suspense_boundary().unwrap();

        let children = driver.children();

        // First always render the children in the background. Rendering the children may cause this boundary to suspend
        let background = suspense_context.under_suspense_boundary(&dom.runtime(), || {
            children.create_with_parents(dom, parent, parent, None)
        });

        driver.store_children(&children);

        // If there are suspended futures, render the fallback

        if !suspense_context.suspended_futures().is_empty() {
            let placeholder_context = suspense_context.clone();
            let (node, nodes_created) =
                suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                    let fallback = driver.fallback();
                    let branch = SuspenseBranch::new(children.as_vnode().clone(), background.mount);
                    store_suspended_branch(driver, dom, &branch);
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
    #[doc(hidden)]
    /// Manually rerun the children of this suspense boundary without diffing against the old nodes.
    ///
    /// This should only be called by dioxus-web after the suspense boundary has been streamed in from the server.
    pub fn resolve_suspense<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        to: &mut M,
        only_write_templates: impl FnOnce(&mut M),
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

            let driver = suspense_driver(dom, scope_id);
            let driver = as_suspense(&driver);

            // Unmount any children to reset any scopes under this suspense boundary
            let children = driver.children();
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
                .expect("suspense placeholders should keep a DOM anchor");
            replace_id_with(to, id, push_replacements);
            currently_rendered.as_vnode().remove_node(
                currently_rendered.root_mount(),
                &mut *dom,
                None,
            );

            // Switch to only writing templates
            only_write_templates(to);

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            let created = suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create_with_parents(dom, parent, parent, Some(to))
            });

            set_rendered_children(driver, dom, scope_id, children, created.mount);

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
    driver: &SuspenseDriver,
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    mut to: Option<&mut dyn WriteMutations>,
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
                            reborrow_writer(&mut to),
                        )
                    });

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
                        new_suspended_mount,
                        scope_id,
                        dom,
                        reborrow_writer(&mut to),
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

                set_rendered_children(driver, dom, scope_id, children, new_mount);
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
                            reborrow_writer(&mut to),
                        );
                        created.mounts[0]
                    });

                // Then diff the new children in the background
                dom.set_mount_mode(old_children_mount, RenderMode::Background);
                let new_children_mount =
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        old_children.mounted_vnode().diff_node(
                            &new_children,
                            dom,
                            reborrow_writer(&mut to),
                        )
                    });

                if suspense_context.suspended_futures().is_empty() {
                    let placeholder_output = MountedOutput::new(new_placeholder, placeholder_mount);
                    promote_resolved_children(
                        driver,
                        &suspense_context,
                        &placeholder_output,
                        new_children,
                        new_children_mount,
                        scope_id,
                        dom,
                        reborrow_writer(&mut to),
                    );
                } else {
                    let branch = SuspenseBranch::new(new_children, new_children_mount);
                    store_suspended_branch(driver, dom, &branch);
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
                    children.as_vnode(),
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

                set_rendered_children(driver, dom, scope_id, children, new_children_mount);

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
    root_mount: MountId,
) {
    driver.store_children(&children);
    dom.scopes[scope_id.index()].last_rendered_node =
        Some(MountedOutput::new(children, root_mount));
}

fn store_suspended_branch(driver: &SuspenseDriver, dom: &mut VirtualDom, branch: &SuspenseBranch) {
    dom.set_mount_mode(branch.root_mount(), RenderMode::Background);
    driver.store_children(&LastRenderedNode::Real(branch.root()));
}

/// Promote freshly-resolved `children` over the visible fallback
/// `placeholder`, record them as the boundary's rendered output, and mark the
/// boundary resolved. Shared by the two diff arms that observe a suspension
/// clearing while a fallback is on screen.
fn promote_resolved_children(
    driver: &SuspenseDriver,
    suspense_context: &SuspenseContext,
    placeholder: &MountedOutput,
    children: VNode,
    children_mount: MountId,
    scope_id: ScopeId,
    dom: &mut VirtualDom,
    to: Option<&mut dyn WriteMutations>,
) {
    dom.set_mount_mode(children_mount, RenderMode::Foreground);
    replace_suspense_nodes(
        suspense_context,
        placeholder,
        &children,
        children_mount,
        dom,
        to,
        |dom| {
            children.remove_node_inner(children_mount, dom, None, false);
        },
    );
    set_rendered_children(
        driver,
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
    children: &VNode,
    children_mount: MountId,
    dom: &mut VirtualDom,
    to: Option<&mut dyn WriteMutations>,
    prepare: impl FnOnce(&mut VirtualDom),
) {
    suspense_context.under_suspense_boundary(&dom.runtime(), || {
        prepare(dom);
        let children = dom
            .current_mounted_view(children_mount)
            .unwrap_or_else(|| children.clone());
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
    mut to: Option<&mut dyn WriteMutations>,
) {
    let parent = dom.mounted_render_parent(placeholder.root_mount());
    if let Some(to_ref) = reborrow_writer(&mut to) {
        if let Some(id) = placeholder.mounted_vnode().mounted_root(0, dom) {
            let child_owns_placeholder_id = (0..children.template.root_count()).any(|root_idx| {
                children
                    .mounted_root(root_idx, dom)
                    .is_some_and(|root_id| root_id == id)
            });

            if !child_owns_placeholder_id {
                replace_id_with(to_ref, id, |to| {
                    let created = dom.create_children_with_parents(
                        Some(to),
                        std::slice::from_ref(children.vnode()),
                        parent,
                        parent,
                    );
                    created.nodes
                });
                placeholder
                    .as_vnode()
                    .remove_node_inner(placeholder.root_mount(), dom, None, true);
                return;
            }
        }
    }

    placeholder.as_vnode().replace(
        placeholder.root_mount(),
        std::slice::from_ref(children.vnode()),
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
    dom.set_mount_mode(mount, RenderMode::Foreground);

    for anchor in vnode.template.anchors() {
        if vnode.dynamic_values[anchor.value_start()]
            .as_node()
            .is_none()
        {
            continue;
        }
        for idx in anchor.values() {
            match vnode.dynamic_values[idx].node() {
                DynamicNode::Component(_) => {
                    let scope_id = dom.unchecked_mounted_dynamic_component_scope(mount, idx);
                    if dom.mark_clean(scope_id) {
                        dom.run_and_diff_scope_with_context(None, scope_id, None);
                    }

                    if let Some(rendered) = dom.scopes[scope_id.index()].last_rendered_node.clone()
                    {
                        promote_suspense_mounts_to_foreground(
                            dom,
                            rendered.as_vnode(),
                            rendered.root_mount(),
                        );
                    }
                }
                DynamicNode::Fragment(nodes) => {
                    let mounts = dom.mounted_fragment_children(mount, idx, nodes.len());
                    for (node, mount) in nodes.iter().zip(mounts) {
                        promote_suspense_mounts_to_foreground(dom, node, mount);
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
