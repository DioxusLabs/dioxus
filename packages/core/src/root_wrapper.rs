use crate::{
    ComponentFunctionExt, Element, ErrorBoundary, Properties, SuspenseBoundary, VComponent, VNode,
    properties::RootProps, view::ViewExt,
};

// We wrap the root scope in a component that renders it inside a default ErrorBoundary and SuspenseBoundary
#[allow(non_snake_case)]
#[allow(clippy::let_and_return)]
pub(crate) fn RootScopeWrapper(props: RootProps<VComponent>) -> Element {
    Element::Ok(
        SuspenseBoundary
            .builder()
            .fallback(|_| Element::Ok(VNode::placeholder()))
            .children(Element::Ok(
                ErrorBoundary
                    .builder()
                    .children(Element::Ok(props.0.into_vnode()))
                    .build()
                    .into_vcomponent(ErrorBoundary)
                    .into_vnode(),
            ))
            .build()
            .into_vcomponent(SuspenseBoundary)
            .into_vnode(),
    )
}
