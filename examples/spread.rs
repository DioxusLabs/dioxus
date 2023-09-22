use dioxus::{
    core::{exports::bumpalo::Bump, Attribute, HasAttributesBox},
    html::{ExtendedGlobalAttributesMarker, GlobalAttributesExtension},
    prelude::*,
};

fn main() {
    let mut dom = VirtualDom::new(app);
    let _ = dom.rebuild();
    let html = dioxus_ssr::render(&dom);

    println!("{}", html);
}

fn app(cx: Scope) -> Element {
    cx.render(::dioxus::core::LazyNodes::new(
        move |__cx: &::dioxus::core::ScopeState| -> ::dioxus::core::VNode {
            static TEMPLATE: ::dioxus::core::Template = ::dioxus::core::Template {
                name: "src/main.rs:15:15:289",
                roots: &[::dioxus::core::TemplateNode::Dynamic { id: 0usize }],
                node_paths: &[&[0u8]],
                attr_paths: &[],
            };
            ::dioxus::core::VNode {
                parent: None,
                key: None,
                template: std::cell::Cell::new(TEMPLATE),
                root_ids: dioxus::core::exports::bumpalo::collections::Vec::with_capacity_in(
                    1usize,
                    __cx.bump(),
                )
                .into(),
                dynamic_nodes: __cx.bump().alloc([__cx.component(
                    Component,
                    Props {
                        bump: __cx.bump(),
                        attributes: Vec::new(),
                    }
                    .width(10)
                    .height("100px"),
                    "Component",
                )]),
                dynamic_attrs: __cx.bump().alloc([]),
            }
        },
    ))
}

#[derive(Props)]
struct Props<'a> {
    bump: &'a Bump,
    attributes: Vec<Attribute<'a>>,
}

impl<'a> HasAttributesBox<'a, Props<'a>> for Props<'a> {
    fn push_attribute(
        mut self,
        name: &'a str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue<'a>,
        volatile: bool,
    ) -> Self {
        self.attributes.push(Attribute {
            name,
            namespace: ns,
            value: attr.into_value(self.bump),
            volatile,
        });
        self
    }
}

impl ExtendedGlobalAttributesMarker for Props<'_> {}

fn Component<'a>(cx: Scope<'a, Props<'a>>) -> Element<'a> {
    let attributes = &cx.props.attributes;
    render! {
        audio {
            ..attributes,
        }
    }
}
