use crate::{
    DynamicNode, DynamicValue, Element, ErrorBoundary, Properties, SuspenseBoundary, Template,
    TemplateOp, TemplatePath, VComponent, VNode, fc_to_builder, properties::RootProps,
};

// We wrap the root scope in a component that renders it inside a default ErrorBoundary and SuspenseBoundary
#[allow(non_snake_case)]
#[allow(clippy::let_and_return)]
pub(crate) fn RootScopeWrapper(props: RootProps<VComponent>) -> Element {
    static TEMPLATE: Template = Template::new(
        &[TemplateOp::text(), TemplateOp::dynamic()],
        &[],
        &[TemplatePath::root(0)],
    );
    Element::Ok(VNode::new(
        None,
        TEMPLATE,
        Box::new([DynamicValue::Node(DynamicNode::Component(
            fc_to_builder(SuspenseBoundary)
                .fallback(|_| Element::Ok(VNode::placeholder()))
                .children(Ok(VNode::new(
                    None,
                    TEMPLATE,
                    Box::new([DynamicValue::Node(DynamicNode::Component({
                        fc_to_builder(ErrorBoundary)
                            .children(Element::Ok(VNode::new(
                                None,
                                TEMPLATE,
                                Box::new([DynamicValue::Node(DynamicNode::Component(props.0))]),
                            )))
                            .build()
                            .into_vcomponent(ErrorBoundary)
                    }))]),
                )))
                .build()
                .into_vcomponent(SuspenseBoundary),
        ))]),
    ))
}
