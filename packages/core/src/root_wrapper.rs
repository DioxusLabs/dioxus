use crate::{
    Element, ErrorBoundary, Properties, SuspenseBoundary, VComponent, VNode,
    error_boundary::ErrorBoundaryProps, properties::RootProps, suspense::SuspenseBoundaryProps,
};

#[cfg(debug_assertions)]
use crate::view::ViewExt;

#[cfg(debug_assertions)]
fn component_vnode(component: VComponent) -> VNode {
    component.into_vnode()
}

#[cfg(not(debug_assertions))]
fn component_vnode(component: VComponent) -> VNode {
    crate::view::into_vnode_with_key_and_capacity::<0, 0, 1, _>(component, None)
}

// We wrap the root scope in a component that renders it inside a default ErrorBoundary and SuspenseBoundary
#[allow(non_snake_case)]
#[allow(clippy::let_and_return)]
pub(crate) fn RootScopeWrapper(props: RootProps<VComponent>) -> Element {
    Element::Ok(component_vnode(
        <SuspenseBoundaryProps as Properties>::builder()
            .fallback(|_| Element::Ok(VNode::placeholder()))
            .children(Element::Ok(component_vnode(
                <ErrorBoundaryProps as Properties>::builder()
                    .children(Element::Ok(component_vnode(props.0)))
                    .build()
                    .into_vcomponent(ErrorBoundary),
            )))
            .build()
            .into_vcomponent(SuspenseBoundary),
    ))
}
