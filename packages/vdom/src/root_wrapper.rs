use crate::{
    Element, ErrorBoundary, Properties, SuspenseBoundary, VComponent, VNode,
    error_boundary::ErrorBoundaryProps, properties::RootProps, suspense::SuspenseBoundaryProps,
};

use crate::view::ViewExt;

fn component_vnode(component: VComponent) -> VNode {
    component.into_vnode()
}

// We wrap the root scope in a component that renders it inside a default ErrorBoundary and SuspenseBoundary
#[allow(non_snake_case)]
#[allow(clippy::let_and_return)]
pub(crate) fn RootScopeWrapper(props: RootProps<VComponent>) -> Element {
    Element::Ok(component_vnode(
        <SuspenseBoundaryProps as Properties>::component_builder(SuspenseBoundary)
            .fallback(|_| Element::Ok(VNode::placeholder()))
            .children(Element::Ok(component_vnode(
                <ErrorBoundaryProps as Properties>::component_builder(ErrorBoundary)
                    .children(Element::Ok(component_vnode(props.0)))
                    .build()
                    .into_vcomponent(),
            )))
            .build()
            .into_vcomponent(),
    ))
}
