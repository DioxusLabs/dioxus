use crate::{prelude::*, properties::RootProps, DynamicNode, VComponent};

// We wrap the root scope in a component that renders it inside a default ErrorBoundary and SuspenseBoundary
#[allow(non_snake_case)]
#[allow(clippy::let_and_return)]
pub(crate) fn RootScopeWrapper(props: RootProps<VComponent>) -> Element {
    static TEMPLATE: Template = Template {
        name: "root_wrapper.rs:16:5:561",
        roots: &[TemplateNode::Dynamic { id: 0usize }],
        node_paths: &[&[0u8]],
        attr_paths: &[],
    };
    Element::Ok(VNode::new(
        None,
        TEMPLATE,
        Box::new([DynamicNode::Component({
            let __props = fc_to_builder(ErrorBoundary)
                .children(Element::Ok(VNode::new(
                    None,
                    TEMPLATE,
                    Box::new([DynamicNode::Component({
                        #[allow(unused_imports)]
                        let __props = fc_to_builder(SuspenseBoundary)
                            .fallback(|_| Element::Ok(VNode::placeholder()))
                            .children(Ok(VNode::new(
                                None,
                                TEMPLATE,
                                Box::new([DynamicNode::Component(props.0)]),
                                Box::new([]),
                            )))
                            .build();
                        crate::VComponent::new(SuspenseBoundary, __props, "SuspenseBoundary")
                    })]),
                    Box::new([]),
                )))
                .build();

            VComponent::new(ErrorBoundary, __props, "ErrorBoundary")
        })]),
        Box::new([]),
    ))
}
