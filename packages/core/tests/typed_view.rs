use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, DynamicValue, VNode,
    view::{IntoChild, TagName, View, attr, attr_dyn, el, text},
};

struct Div;
impl TagName for Div {
    const NAME: &'static str = "div";
}

struct H2;
impl TagName for H2 {
    const NAME: &'static str = "h2";
}

struct P;
impl TagName for P {
    const NAME: &'static str = "p";
}

struct Span;
impl TagName for Span {
    const NAME: &'static str = "span";
}

dioxus_core::static_attribute!(CardClass, "class", "card");
dioxus_core::static_attribute!(BadgeClass, "class", "badge");
dioxus_core::static_attribute!(TitleRole, "data-role", "title");
dioxus_core::static_text!(TitlePrefix, "Title: ");
dioxus_core::static_text!(HelloPrefix, "Hello, ");

fn badge<Content, Marker>(content: Content) -> impl View
where
    Content: IntoChild<Marker>,
{
    el::<Span>().attr(attr::<BadgeClass>()).child(content)
}

fn card(style: &str, title: &str, name: &str) -> impl View {
    el::<Div>()
        .attr(attr::<CardClass>())
        .attr(attr_dyn("style", style, None, false))
        .child(
            el::<H2>()
                .attr(attr::<TitleRole>())
                .child(text::<TitlePrefix>())
                .child(title.to_string()),
        )
        .child(
            el::<P>()
                .attr(attr::<CardClass>())
                .child(text::<HelloPrefix>())
                .child(badge(name.to_string())),
        )
}

#[test]
fn view_builder_pushes_dynamic_values_in_template_order() {
    let vnode = card("color: crimson", "Welcome", "Ada").into_vnode();

    assert_eq!(vnode.template.dynamics().len(), 3);
    assert_eq!(vnode.dynamic_values.len(), 3);
    assert_eq!(vnode.template.root_count(), 1);

    match &vnode.dynamic_values[..] {
        [
            DynamicValue::Attrs(attrs),
            DynamicValue::Node(DynamicNode::Text(title)),
            DynamicValue::Node(DynamicNode::Text(name)),
        ] => {
            assert_eq!(attrs[0].name, "style");
            assert_eq!(
                attrs[0].value,
                AttributeValue::Text("color: crimson".to_string())
            );
            assert_eq!(title.value, "Welcome");
            assert_eq!(name.value, "Ada");
        }
        other => panic!("unexpected dynamic values: {other:?}"),
    }
}

#[test]
fn html_view_helpers_create_typed_elements() {
    let vnode = dioxus_html::div()
        .attr(attr_dyn("class", "panel", None, false))
        .attr(dioxus_core::static_attribute!("data-inline", "true"))
        .child(dioxus_html::span().child("hello"))
        .into_vnode();

    assert_eq!(vnode.template.root_count(), 1);
    assert_eq!(vnode.template.dynamics().len(), 2);
}

#[test]
fn html_builder_static_value_creates_static_attribute() {
    let vnode = dioxus_html::ImgExtension::height(
        dioxus_html::ImgExtension::width(dioxus_html::img(), dioxus_core::static_value!("320")),
        dioxus_core::static_value!("180"),
    )
    .into_vnode();

    assert_eq!(vnode.template.root_count(), 1);
    assert_eq!(vnode.template.dynamics().len(), 0);
    assert_eq!(vnode.dynamic_values.len(), 0);

    let attrs = static_attrs(&vnode);
    assert!(attrs.contains(&("width", "320", None)));
    assert!(attrs.contains(&("height", "180", None)));
}

#[test]
fn html_builder_static_value_preserves_descriptor_name_and_namespace() {
    let vnode = dioxus_html::GlobalAttributesExtension::background_color(
        dioxus_html::div(),
        dioxus_core::static_value!("red"),
    )
    .into_vnode();

    assert_eq!(vnode.template.dynamics().len(), 0);
    assert_eq!(vnode.dynamic_values.len(), 0);
    assert!(static_attrs(&vnode).contains(&("background-color", "red", Some("style"))));
}

#[test]
fn html_builder_string_value_remains_dynamic_attribute() {
    let vnode = dioxus_html::ImgExtension::width(dioxus_html::img(), "320").into_vnode();
    let attr = only_dynamic_attr(&vnode);

    assert_eq!(vnode.template.dynamics().len(), 1);
    assert_eq!(attr.name, "width");
    assert_eq!(attr.namespace, None);
    assert_eq!(attr.value, AttributeValue::Text("320".to_string()));
}

#[test]
fn html_builder_numeric_value_remains_dynamic_attribute() {
    let vnode = dioxus_html::ImgExtension::width(dioxus_html::img(), 320usize).into_vnode();
    let attr = only_dynamic_attr(&vnode);

    assert_eq!(vnode.template.dynamics().len(), 1);
    assert_eq!(attr.name, "width");
    assert_eq!(attr.namespace, None);
    assert_eq!(attr.value, AttributeValue::Int(320));
}

fn static_attrs(vnode: &VNode) -> Vec<(&'static str, &'static str, Option<&'static str>)> {
    vnode
        .template
        .ops()
        .iter()
        .enumerate()
        .filter_map(|(op, _)| vnode.template.static_attr_at_op(op))
        .collect()
}

fn only_dynamic_attr(vnode: &VNode) -> &Attribute {
    match &vnode.dynamic_values[..] {
        [DynamicValue::Attrs(attrs)] if attrs.len() == 1 => &attrs[0],
        other => panic!("expected one dynamic attribute: {other:?}"),
    }
}
