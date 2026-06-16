use dioxus_core::internal::HotReloadDynamicSlot;
use dioxus_core::{Template, TemplateRawAttrNamespace, TemplateRawOp};
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::*;
use internment::Intern;
use std::hash::Hash;

// interns a object into a static object, reusing the value if it already exists
pub(crate) fn intern<T: Eq + Hash + Send + Sync + ?Sized + 'static>(
    s: impl Into<Intern<T>>,
) -> &'static T {
    s.into().as_ref()
}

pub(crate) fn html_tag_and_namespace<Ctx: HotReloadingContext>(
    attr: &Attribute,
) -> (&'static str, Option<&'static str>) {
    let attribute_name_rust = attr.name.to_string();
    let element_name = attr.el_name.as_ref().unwrap();
    let rust_name = match element_name {
        ElementName::Ident(i) => i.to_string(),
        // If this is a web component, just use the name of the elements instead of mapping the attribute
        // through the hot reloading context
        ElementName::Custom(_) => return (intern(attribute_name_rust.as_str()), None),
    };

    Ctx::map_attribute(&rust_name, &attribute_name_rust)
        .unwrap_or((intern(attribute_name_rust.as_str()), None))
}

pub(crate) struct HotReloadTemplateParts<'a> {
    pub(crate) template: Template,
    pub(crate) dynamic_slots: Vec<HotReloadDynamicSlot>,
    pub(crate) dynamic_nodes: Vec<&'a BodyNode>,
    pub(crate) dynamic_attributes: Vec<&'a Attribute>,
}

pub(crate) fn hot_reload_template_parts<'a, Ctx: HotReloadingContext>(
    body: &'a TemplateBody,
) -> Option<HotReloadTemplateParts<'a>> {
    let mut builder = NativeTemplateBuilder::default();
    builder.visit_roots::<Ctx>(&body.roots)?;

    let raw_ops = intern(&*builder.raw_ops);

    Some(HotReloadTemplateParts {
        template: Template::from_raw_ops(raw_ops),
        dynamic_slots: builder.dynamic_slots,
        dynamic_nodes: builder.dynamic_nodes,
        dynamic_attributes: builder.dynamic_attributes,
    })
}

#[derive(Default)]
struct NativeTemplateBuilder<'a> {
    raw_ops: Vec<TemplateRawOp>,
    dynamic_slots: Vec<HotReloadDynamicSlot>,
    dynamic_nodes: Vec<&'a BodyNode>,
    dynamic_attributes: Vec<&'a Attribute>,
    next_dynamic_node: usize,
    next_dynamic_attr: usize,
}

impl<'a> NativeTemplateBuilder<'a> {
    fn visit_roots<Ctx: HotReloadingContext>(&mut self, nodes: &'a [BodyNode]) -> Option<()> {
        for node in nodes {
            self.visit_node::<Ctx>(node)?;
        }
        Some(())
    }

    fn visit_node<Ctx: HotReloadingContext>(&mut self, node: &'a BodyNode) -> Option<()> {
        match node {
            BodyNode::Element(element) => self.visit_element::<Ctx>(element),
            BodyNode::Text(text) => match text.input.to_static() {
                Some(text) => {
                    self.raw_ops
                        .push(TemplateRawOp::static_text(intern(text.as_str())));
                    Some(())
                }
                None => self.push_dynamic_node(node),
            },
            BodyNode::RawExpr(_)
            | BodyNode::Component(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_) => self.push_dynamic_node(node),
        }
    }

    fn visit_element<Ctx: HotReloadingContext>(&mut self, element: &'a Element) -> Option<()> {
        let rust_name = element.name.to_string();
        let (tag, namespace) =
            Ctx::map_element(&rust_name).unwrap_or((intern(rust_name.as_str()), None));

        self.raw_ops
            .push(TemplateRawOp::open_element(tag, namespace));

        for attr in &element.merged_attributes {
            self.push_attribute::<Ctx>(attr)?;
        }

        for child in &element.children {
            self.visit_node::<Ctx>(child)?;
        }

        self.raw_ops.push(TemplateRawOp::close_element());
        Some(())
    }

    fn push_attribute<Ctx: HotReloadingContext>(&mut self, attr: &'a Attribute) -> Option<()> {
        let Some((_, value)) = attr.as_static_str_literal() else {
            let id = self.next_dynamic_attr;
            self.next_dynamic_attr += 1;
            self.raw_ops.push(TemplateRawOp::dynamic_attr());
            self.dynamic_slots.push(HotReloadDynamicSlot::Attribute(id));
            self.dynamic_attributes.push(attr);
            return Some(());
        };

        let (name, namespace) = html_tag_and_namespace::<Ctx>(attr);
        let namespace = match namespace {
            Some("style") => TemplateRawAttrNamespace::Style,
            Some(namespace) => TemplateRawAttrNamespace::Custom(namespace),
            None => TemplateRawAttrNamespace::None,
        };
        self.raw_ops.push(TemplateRawOp::StaticAttr {
            name,
            value: intern(value.to_static().unwrap().as_str()),
            namespace,
        });
        Some(())
    }

    fn push_dynamic_node(&mut self, node: &'a BodyNode) -> Option<()> {
        let id = self.next_dynamic_node;
        self.next_dynamic_node += 1;
        self.raw_ops.push(TemplateRawOp::dynamic_node());
        self.dynamic_slots.push(HotReloadDynamicSlot::Node(id));
        self.dynamic_nodes.push(node);
        Some(())
    }
}
