use dioxus::core::view::View;
use dioxus::prelude::*;

#[test]
fn component_function_builder_works_for_component_macro_props() {
    #[component]
    fn Greeting(#[props(into)] name: String, excited: bool) -> Element {
        let _ = (name, excited);
        VNode::empty()
    }

    let props = Greeting.builder().name("Ada").excited(true).build();
    let component = Greeting.with_props(props);

    assert!(component.name.contains("Greeting"));
}

#[test]
fn component_function_builder_works_for_manual_props() {
    #[derive(Clone, PartialEq, Props)]
    struct ManualProps {
        count: usize,
    }

    #[allow(non_snake_case)]
    fn Manual(props: ManualProps) -> Element {
        let _ = props.count;
        VNode::empty()
    }

    let props = Manual.builder().count(7).build();
    let component = Manual.with_props(props);

    assert!(component.name.contains("Manual"));
}

#[test]
fn component_function_can_be_used_as_typed_view_child() {
    #[component]
    fn Label(#[props(into)] text: String) -> Element {
        Ok(dioxus::html::span().child(text).into_vnode())
    }

    let label = || Label.with_props(Label.builder().text("Ada").build());
    let root = dioxus::html::div()
        .child(label())
        .child([label()].into_iter())
        .into_vnode();

    assert_eq!(
        dioxus_ssr::render_element(Ok(root)),
        "<div><span>Ada</span><span>Ada</span></div>"
    );
}
