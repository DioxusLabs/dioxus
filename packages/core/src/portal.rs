use crate::{
    RenderTargetId,
    any_props::AnyProps,
    diff::anchor::{Anchor, create_at_anchor_with_parents},
    innerlude::*,
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
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub struct PortalPropsBuilder<TypedBuilderFields> {
    fields: TypedBuilderFields,
    _phantom: (),
}

impl Properties for PortalProps {
    type Builder = PortalPropsBuilder<((), ())>;

    fn builder() -> Self::Builder {
        PortalProps::builder()
    }

    fn memoize(&mut self, new: &Self) -> bool {
        let equal = self == new;
        if !equal {
            self.target = new.target;
            self.children = new.children.clone();
        }
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

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum PortalPropsBuilder_Error_Repeated_field_target {}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__children> PortalPropsBuilder<((RenderTargetId,), __children)> {
    #[deprecated(note = "Repeated field target")]
    #[allow(clippy::type_complexity)]
    pub fn target(
        self,
        _: PortalPropsBuilder_Error_Repeated_field_target,
    ) -> PortalPropsBuilder<((RenderTargetId,), __children)> {
        self
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

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum PortalPropsBuilder_Error_Repeated_field_children {}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl<__target> PortalPropsBuilder<(__target, (Element,))> {
    #[deprecated(note = "Repeated field children")]
    #[allow(clippy::type_complexity)]
    pub fn children(
        self,
        _: PortalPropsBuilder_Error_Repeated_field_children,
    ) -> PortalPropsBuilder<(__target, (Element,))> {
        self
    }
}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum PortalPropsBuilder_Error_Missing_required_field_target {}

#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
impl<__children> PortalPropsBuilder<((), __children)> {
    #[deprecated(note = "Missing required field target")]
    pub fn build(self, _: PortalPropsBuilder_Error_Missing_required_field_target) -> PortalProps {
        panic!()
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

/// Render children into another render target while keeping logical ancestry.
#[allow(non_snake_case)]
pub fn Portal(__props: PortalProps) -> Element {
    unreachable!("Portal should not be called directly")
}

impl PortalProps {
    pub(crate) fn downcast_from_props(props: &mut dyn AnyProps) -> Option<&mut Self> {
        props.props_mut().downcast_mut()
    }

    pub(crate) fn create<M: WriteMutations>(
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
    ) -> usize {
        let target_id = component
            .props
            .props()
            .downcast_ref::<PortalProps>()
            .expect("Portal props should downcast")
            .target;
        let mut scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));

        if scope_id.is_placeholder() {
            let scope_state = dom.runtime.clone().with_render_target(target_id, || {
                dom.new_scope(component.props.duplicate(), component.name)
                    .state()
                    .id
            });
            scope_id = scope_state;
            dom.set_mounted_dyn_node(mount, idx, scope_id.0);
        }

        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope_state = &mut dom.scopes[scope_id.0];
            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
            let children = props.children.clone();
            let target_id = props.target;

            dom.runtime.clone().with_render_target(target_id, || {
                let render_to = to.filter(|_| dom.render_target_should_write(target_id));
                let should_mount = render_to.is_some();
                create_at_anchor_with_parents(
                    std::slice::from_ref(children.as_vnode()),
                    None,
                    parent,
                    Anchor::AppendTo(ElementId::ROOT),
                    dom,
                    render_to,
                );
                dom.scopes[scope_id.0].last_rendered_node = Some(children);
                if should_mount {
                    dom.runtime.get_state(scope_id).mount(&dom.runtime);
                }
                0
            })
        })
    }

    pub(crate) fn diff<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        mut to: Option<&mut M>,
    ) {
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            let scope_state = &mut dom.scopes[scope_id.0];
            let props = Self::downcast_from_props(&mut *scope_state.props).unwrap();
            let new_children = props.children.clone();
            let target_id = props.target;
            let old_children = dom.scopes[scope_id.0].last_rendered_node.take().unwrap();
            let old_target_id = dom.runtime.get_state(scope_id).target_id();

            if old_target_id != target_id {
                let logical_parent =
                    old_children
                        .as_vnode()
                        .mount
                        .get()
                        .as_usize()
                        .and_then(|mount| {
                            dom.runtime
                                .fibers
                                .borrow()
                                .get(mount)
                                .and_then(|fiber| fiber.logical_parent)
                        });

                dom.runtime.clone().with_render_target(old_target_id, || {
                    let render_to = to
                        .as_deref_mut()
                        .filter(|_| dom.render_target_should_write(old_target_id));
                    old_children.remove_node_inner(dom, render_to, true);
                });

                if let Some(scope) = dom.runtime.scope_states.borrow_mut()[scope_id.0].as_mut() {
                    scope.set_target_id(target_id);
                }

                dom.runtime.clone().with_render_target(target_id, || {
                    let render_to = to.filter(|_| dom.render_target_should_write(target_id));
                    let should_mount = render_to.is_some();
                    create_at_anchor_with_parents(
                        std::slice::from_ref(new_children.as_vnode()),
                        None,
                        logical_parent,
                        Anchor::AppendTo(ElementId::ROOT),
                        dom,
                        render_to,
                    );
                    dom.scopes[scope_id.0].last_rendered_node = Some(new_children);
                    if should_mount {
                        dom.runtime.get_state(scope_id).mount(&dom.runtime);
                    }
                });
                return;
            }

            dom.runtime.clone().with_render_target(target_id, || {
                let mut render_to = to
                    .filter(|_| dom.runtime.scope_should_render(scope_id))
                    .filter(|_| dom.render_target_should_write(target_id));
                old_children.diff_node(&new_children, dom, render_to.as_deref_mut());
                dom.scopes[scope_id.0].last_rendered_node = Some(new_children);
                if render_to.is_some() {
                    dom.runtime.get_state(scope_id).mount(&dom.runtime);
                }
            });
        })
    }

    pub(crate) fn remove<M: WriteMutations>(
        scope_id: ScopeId,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
        destroy_component_state: bool,
    ) {
        let target_id = dom.runtime.get_state(scope_id).target_id();
        dom.runtime.clone().with_scope_on_stack(scope_id, || {
            dom.runtime.clone().with_render_target(target_id, || {
                let render_to = to.filter(|_| dom.render_target_should_write(target_id));
                if let Some(node) = dom.scopes[scope_id.0].last_rendered_node.clone() {
                    node.remove_node_inner(dom, render_to, destroy_component_state);
                }
            });
        });

        if destroy_component_state {
            dom.drop_scope(scope_id);
        }
    }
}
