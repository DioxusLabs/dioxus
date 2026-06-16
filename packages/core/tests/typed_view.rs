use dioxus_core::{
    AttributeValue, DynamicNode, DynamicValue,
    view::{IntoChild, StaticAttribute, TagName, View, attr, attr_dyn, el, text},
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

struct CardClass;
impl StaticAttribute for CardClass {
    const NAME: &'static str = "class";
    const VALUE: &'static str = "card";
}

struct BadgeClass;
impl StaticAttribute for BadgeClass {
    const NAME: &'static str = "class";
    const VALUE: &'static str = "badge";
}

struct TitleRole;
impl StaticAttribute for TitleRole {
    const NAME: &'static str = "data-role";
    const VALUE: &'static str = "title";
}

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
        .child(dioxus_html::span().child("hello"))
        .into_vnode();

    assert_eq!(vnode.template.root_count(), 1);
    assert_eq!(vnode.template.dynamics().len(), 2);
}
