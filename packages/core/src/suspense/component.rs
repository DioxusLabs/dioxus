use crate::{innerlude::*, scope_context::SuspenseLocation};

/// Properties for the [`SuspenseBoundary()`] component.
#[allow(non_camel_case_types)]
pub struct SuspenseBoundaryProps {
    fallback: Callback<SuspenseContext, Element>,
    /// The children of the suspense boundary
    children: Element,
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
        self.fallback.__set(new.fallback.__take());
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
        component_name: &'static str,
    ) -> VComponent {
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
        let fallback = fallback.0;
        let children = SuspenseBoundaryPropsBuilder_Optional::into_value(children, VNode::empty);
        SuspenseBoundaryPropsWithOwner {
            inner: SuspenseBoundaryProps { fallback, children },
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
///             fallback: |context: SuspenseContext| rsx! {
///                 if let Some(placeholder) = context.suspense_placeholder() {
///                     {placeholder}
///                 } else {
///                     "Loading..."
///                 }
///             },
///             Article {}
///         }
///     }
/// }
/// ```
#[allow(non_snake_case)]
pub fn SuspenseBoundary(mut __props: SuspenseBoundaryProps) -> Element {
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
use generational_box::Owner;
#[allow(unused)]
pub use SuspenseBoundary_completions::Component::SuspenseBoundary;

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
        let mut scope_id = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
        // If the ScopeId is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
        if scope_id.is_placeholder() {
            {
                let suspense_context = SuspenseContext::new();

                let suspense_boundary_location =
                    crate::scope_context::SuspenseLocation::SuspenseBoundary(
                        suspense_context.clone(),
                    );
                dom.runtime
                    .clone()
                    .with_suspense_location(suspense_boundary_location, || {
                        let scope_state = dom
                            .new_scope(component.props.duplicate(), component.name)
                            .state();
                        suspense_context.mount(scope_state.id);
                        scope_id = scope_state.id;
                    });
            }

            // Store the scope id for the next render
            dom.mounts[mount.0].mounted_dynamic_nodes[idx] = scope_id.0;
        }
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope_state = &mut dom.scopes[scope_id.0];
            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
            let suspense_context =
                SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                    .unwrap();

            let children = RenderReturn {
                node: props
                    .children
                    .as_ref()
                    .map(|node| node.clone_mounted())
                    .map_err(Clone::clone),
            };

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create(dom, parent, None::<&mut M>);
            });

            // Store the (now mounted) children back into the scope state
            let scope_state = &mut dom.scopes[scope_id.0];
            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
            props.children = children.clone().node;

            let scope_state = &mut dom.scopes[scope_id.0];
            let suspense_context = scope_state
                .state()
                .suspense_location()
                .suspense_context()
                .unwrap()
                .clone();
            // If there are suspended futures, render the fallback
            let nodes_created = if !suspense_context.suspended_futures().is_empty() {
                let (node, nodes_created) =
                    suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                        let scope_state = &mut dom.scopes[scope_id.0];
                        let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
                        let suspense_context =
                            SuspenseContext::downcast_suspense_boundary_from_scope(
                                &dom.runtime,
                                scope_id,
                            )
                            .unwrap();
                        suspense_context.set_suspended_nodes(children.into());
                        let suspense_placeholder = props.fallback.call(suspense_context);
                        let node = RenderReturn {
                            node: suspense_placeholder,
                        };
                        let nodes_created = node.create(dom, parent, to);
                        (node, nodes_created)
                    });

                let scope_state = &mut dom.scopes[scope_id.0];
                scope_state.last_rendered_node = Some(node);

                nodes_created
            } else {
                // Otherwise just render the children in the real dom
                debug_assert!(children.mount.get().mounted());
                let nodes_created = suspense_context
                    .under_suspense_boundary(&dom.runtime(), || children.create(dom, parent, to));
                let scope_state = &mut dom.scopes[scope_id.0];
                scope_state.last_rendered_node = Some(children);
                let suspense_context =
                    SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                        .unwrap();
                suspense_context.take_suspended_nodes();
                mark_suspense_resolved(dom, scope_id);

                nodes_created
            };
            nodes_created
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
            let suspense_context = scope_state
                .state()
                .suspense_location()
                .suspense_context()
                .unwrap()
                .clone();
            suspense_context.inner.suspended_tasks.borrow_mut().clear();

            // Get the parent of the suspense boundary to later create children with the right parent
            let currently_rendered = scope_state.last_rendered_node.as_ref().unwrap().clone();
            let mount = currently_rendered.mount.get();
            let parent = dom
                .mounts
                .get(mount.0)
                .expect("suspense placeholder is not mounted")
                .parent;

            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();

            // Unmount any children to reset any scopes under this suspense boundary
            let children = props
                .children
                .as_ref()
                .map(|node| node.clone_mounted())
                .map_err(Clone::clone);
            let suspense_context =
                SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                    .unwrap();
            let suspended = suspense_context.suspended_nodes();
            if let Some(node) = suspended {
                node.remove_node(&mut *dom, None::<&mut M>, None);
            }
            // Replace the rendered nodes with resolved nodes
            currently_rendered.remove_node(&mut *dom, Some(to), Some(replace_with));

            // Switch to only writing templates
            only_write_templates(to);

            let children = RenderReturn { node: children };
            children.mount.take();

            // First always render the children in the background. Rendering the children may cause this boundary to suspend
            suspense_context.under_suspense_boundary(&dom.runtime(), || {
                children.create(dom, parent, Some(to));
            });

            // Store the (now mounted) children back into the scope state
            let scope_state = &mut dom.scopes[scope_id.0];
            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
            props.children = children.clone().node;
            scope_state.last_rendered_node = Some(children);
            let suspense_context =
                SuspenseContext::downcast_suspense_boundary_from_scope(&dom.runtime, scope_id)
                    .unwrap();
            suspense_context.take_suspended_nodes();
        })
    }

    pub(crate) fn diff<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
    ) {
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope = &mut dom.scopes[scope_id.0];
            let myself = Self::downcast_from_props(&mut *scope.props)
                .unwrap()
                .clone();

            let last_rendered_node = scope.last_rendered_node.as_ref().unwrap().clone_mounted();

            let Self {
                fallback, children, ..
            } = myself;

            let suspense_context = scope.state().suspense_boundary().unwrap().clone();
            let suspended_nodes = suspense_context.suspended_nodes();
            let suspended = !suspense_context.suspended_futures().is_empty();
            match (suspended_nodes, suspended) {
                // We already have suspended nodes that still need to be suspended
                // Just diff the normal and suspended nodes
                (Some(suspended_nodes), true) => {
                    let new_suspended_nodes: VNode = RenderReturn { node: children }.into();

                    // Diff the placeholder nodes in the dom
                    let new_placeholder =
                        suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                            let old_placeholder = last_rendered_node;
                            let new_placeholder = RenderReturn {
                                node: fallback.call(suspense_context.clone()),
                            };

                            old_placeholder.diff_node(&new_placeholder, dom, to);
                            new_placeholder
                        });

                    // Set the last rendered node to the placeholder
                    dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);

                    // Diff the suspended nodes in the background
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        suspended_nodes.diff_node(&new_suspended_nodes, dom, None::<&mut M>);
                    });

                    let suspense_context = SuspenseContext::downcast_suspense_boundary_from_scope(
                        &dom.runtime,
                        scope_id,
                    )
                    .unwrap();
                    suspense_context.set_suspended_nodes(new_suspended_nodes);
                }
                // We have no suspended nodes, and we are not suspended. Just diff the children like normal
                (None, false) => {
                    let old_children = last_rendered_node;
                    let new_children = RenderReturn { node: children };

                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        old_children.diff_node(&new_children, dom, to);
                    });

                    // Set the last rendered node to the new children
                    dom.scopes[scope_id.0].last_rendered_node = Some(new_children);
                }
                // We have no suspended nodes, but we just became suspended. Move the children to the background
                (None, true) => {
                    let old_children = last_rendered_node;
                    let new_children: VNode = RenderReturn { node: children }.into();

                    let new_placeholder = RenderReturn {
                        node: fallback.call(suspense_context.clone()),
                    };

                    // Move the children to the background
                    let mount = old_children.mount.get();
                    let mount = dom.mounts.get(mount.0).expect("mount should exist");
                    let parent = mount.parent;

                    suspense_context.in_suspense_placeholder(&dom.runtime(), || {
                        old_children.move_node_to_background(
                            std::slice::from_ref(&*new_placeholder),
                            parent,
                            dom,
                            to,
                        );
                    });

                    // Then diff the new children in the background
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        old_children.diff_node(&new_children, dom, None::<&mut M>);
                    });

                    // Set the last rendered node to the new suspense placeholder
                    dom.scopes[scope_id.0].last_rendered_node = Some(new_placeholder);

                    let suspense_context = SuspenseContext::downcast_suspense_boundary_from_scope(
                        &dom.runtime,
                        scope_id,
                    )
                    .unwrap();
                    suspense_context.set_suspended_nodes(new_children);

                    un_resolve_suspense(dom, scope_id);
                } // We have suspended nodes, but we just got out of suspense. Move the suspended nodes to the foreground
                (Some(old_suspended_nodes), false) => {
                    let old_placeholder = last_rendered_node;
                    let new_children = RenderReturn { node: children };

                    // First diff the two children nodes in the background
                    suspense_context.under_suspense_boundary(&dom.runtime(), || {
                        old_suspended_nodes.diff_node(&new_children, dom, None::<&mut M>);

                        // Then replace the placeholder with the new children
                        let mount = old_placeholder.mount.get();
                        let mount = dom.mounts.get(mount.0).expect("mount should exist");
                        let parent = mount.parent;
                        old_placeholder.replace(
                            std::slice::from_ref(&*new_children),
                            parent,
                            dom,
                            to,
                        );
                    });

                    // Set the last rendered node to the new children
                    dom.scopes[scope_id.0].last_rendered_node = Some(new_children);

                    let suspense_context = SuspenseContext::downcast_suspense_boundary_from_scope(
                        &dom.runtime,
                        scope_id,
                    )
                    .unwrap();
                    suspense_context.take_suspended_nodes();

                    mark_suspense_resolved(dom, scope_id);
                }
            }
        })
    }
}

/// Move to a resolved suspense state
fn mark_suspense_resolved(dom: &mut VirtualDom, scope_id: ScopeId) {
    dom.resolved_scopes.push(scope_id);
}

/// Move from a resolved suspense state to an suspended state
fn un_resolve_suspense(dom: &mut VirtualDom, scope_id: ScopeId) {
    dom.resolved_scopes.retain(|&id| id != scope_id);
}

impl SuspenseContext {
    /// Run a closure under a suspense boundary
    pub fn under_suspense_boundary<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(SuspenseLocation::UnderSuspense(self.clone()), f)
    }

    /// Run a closure under a suspense placeholder
    pub fn in_suspense_placeholder<O>(&self, runtime: &Runtime, f: impl FnOnce() -> O) -> O {
        runtime.with_suspense_location(SuspenseLocation::InSuspensePlaceholder(self.clone()), f)
    }

    /// Try to get a suspense boundary from a scope id
    pub fn downcast_suspense_boundary_from_scope(
        runtime: &Runtime,
        scope_id: ScopeId,
    ) -> Option<Self> {
        runtime
            .get_state(scope_id)
            .and_then(|scope| scope.suspense_boundary())
    }

    pub(crate) fn remove_suspended_nodes<M: WriteMutations>(
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
            node.remove_node_inner(dom, None::<&mut M>, destroy_component_state, None)
        }
    }
}
