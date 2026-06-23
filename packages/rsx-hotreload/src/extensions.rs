use dioxus_core::Template;
use dioxus_core_template::RuntimeTemplateBuilder;
use dioxus_core_types::HotReloadingContext;
use dioxus_rsx::*;
use internment::Intern;
use std::hash::Hash;
use std::marker::PhantomData;

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
        // Strip any `r#` so the mapped name matches what the compiled binary registered (codegen
        // resolves the tag through the same `tag_name_string`).
        ElementName::Ident(_) => element_name.tag_name_string(),
        // If this is a web component, just use the name of the elements instead of mapping the attribute
        // through the hot reloading context
        ElementName::Custom(_) => return (intern(attribute_name_rust.as_str()), None),
    };

    Ctx::map_attribute(&rust_name, &attribute_name_rust)
        .unwrap_or((intern(attribute_name_rust.as_str()), None))
}

pub(crate) struct HotReloadTemplateParts<'a> {
    pub(crate) template: Template,
    pub(crate) dynamic_nodes: Vec<&'a BodyNode>,
    pub(crate) dynamic_attributes: Vec<&'a Attribute>,
}

pub(crate) fn hot_reload_template_parts<'a, Ctx: HotReloadingContext>(
    body: &'a TemplateBody,
) -> Option<HotReloadTemplateParts<'a>> {
    let mut builder = NativeTemplateBuilder::<'a, Ctx> {
        template: RuntimeTemplateBuilder::default(),
        dynamic_nodes: Vec::new(),
        dynamic_attributes: Vec::new(),
        following_static_at_parent: false,
        _ctx: PhantomData,
    };
    // Walk in canonical fill order so the dynamic slots line up with the runtime VNode built by
    // the typed view builder.
    visit_roots(&mut builder, &body.roots)?;

    let NativeTemplateBuilder {
        template,
        dynamic_nodes,
        dynamic_attributes,
        ..
    } = builder;

    Some(HotReloadTemplateParts {
        template: template.finish(),
        dynamic_nodes,
        dynamic_attributes,
    })
}

struct NativeTemplateBuilder<'a, Ctx> {
    template: RuntimeTemplateBuilder,
    dynamic_nodes: Vec<&'a BodyNode>,
    dynamic_attributes: Vec<&'a Attribute>,
    following_static_at_parent: bool,
    _ctx: PhantomData<Ctx>,
}

impl<'a, Ctx: HotReloadingContext> FillOrderVisitor<'a> for NativeTemplateBuilder<'a, Ctx> {
    fn visit_siblings(&mut self, nodes: &'a [BodyNode]) -> Option<()> {
        for (index, node) in nodes.iter().enumerate() {
            let previous = self.following_static_at_parent;
            self.following_static_at_parent = siblings_have_static_node(nodes, index + 1);
            let result = FillOrderVisitor::visit_node(self, node);
            self.following_static_at_parent = previous;
            result?;
        }
        Some(())
    }

    fn open_element(&mut self, element: &'a Element) -> Option<()> {
        // Use `tag_name_string` (the same resolution codegen uses) so raw-ident elements like
        // `r#use` map to the bare name the compiled binary registered, not `r#use`.
        let rust_name = element.name.tag_name_string();
        let (tag, namespace) =
            Ctx::map_element(&rust_name).unwrap_or((intern(rust_name.as_str()), None));
        self.template.open_element(tag, namespace);
        Some(())
    }

    fn close_element(&mut self, _element: &'a Element) -> Option<()> {
        self.template.close_element();
        Some(())
    }

    fn static_attribute(&mut self, _element: &'a Element, attr: &'a Attribute) -> Option<()> {
        // Emitted before children: a static attribute is lowered immediately, into the op slots
        // that precede the element's child nodes.
        let (_, value) = attr.as_static_str_literal()?;
        let (name, namespace) = html_tag_and_namespace::<Ctx>(attr);
        self.template
            .static_attr(name, intern(value.to_static().unwrap().as_str()), namespace);
        Some(())
    }

    fn dynamic_attribute(&mut self, _element: &'a Element, attr: &'a Attribute) -> Option<()> {
        self.template.dynamic_attr();
        self.dynamic_attributes.push(attr);
        Some(())
    }

    fn static_text(&mut self, text: &'a TextNode) -> Option<()> {
        let value = text.input.to_static()?;
        self.template.static_text(intern(value.as_str()));
        Some(())
    }

    fn dynamic_node(&mut self, node: &'a BodyNode) -> Option<()> {
        self.template.dynamic_node(self.following_static_at_parent);
        self.dynamic_nodes.push(node);
        Some(())
    }
}

fn siblings_have_static_node(nodes: &[BodyNode], start: usize) -> bool {
    nodes[start..].iter().any(node_has_static_root)
}

fn node_has_static_root(node: &BodyNode) -> bool {
    match node {
        BodyNode::Element(_) => true,
        BodyNode::Text(text) => text.is_static(),
        BodyNode::RawExpr(_)
        | BodyNode::Component(_)
        | BodyNode::ForLoop(_)
        | BodyNode::IfChain(_)
        | BodyNode::SyntheticBoundary(_) => false,
    }
}
