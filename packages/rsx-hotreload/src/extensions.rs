use dioxus_core::Template;
use dioxus_core::internal::HotReloadDynamicSlot;
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
        ElementName::Path(p) => p.segments.last().unwrap().ident.to_string(),
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
    let mut builder = NativeTemplateBuilder::<'a, Ctx> {
        template: RuntimeTemplateBuilder::default(),
        dynamic_slots: Vec::new(),
        dynamic_nodes: Vec::new(),
        dynamic_attributes: Vec::new(),
        _ctx: PhantomData,
    };
    // Walk in canonical fill order (children, then dynamic attributes, then key) so the dynamic
    // slots line up with the runtime VNode built by the typed view builder.
    visit_roots(&mut builder, &body.roots)?;

    let NativeTemplateBuilder {
        template,
        dynamic_slots,
        dynamic_nodes,
        dynamic_attributes,
        ..
    } = builder;

    Some(HotReloadTemplateParts {
        template: template.finish(),
        dynamic_slots,
        dynamic_nodes,
        dynamic_attributes,
    })
}

struct NativeTemplateBuilder<'a, Ctx> {
    template: RuntimeTemplateBuilder,
    dynamic_slots: Vec<HotReloadDynamicSlot>,
    dynamic_nodes: Vec<&'a BodyNode>,
    dynamic_attributes: Vec<&'a Attribute>,
    _ctx: PhantomData<Ctx>,
}

impl<'a, Ctx: HotReloadingContext> FillOrderVisitor<'a> for NativeTemplateBuilder<'a, Ctx> {
    fn open_element(&mut self, element: &'a Element) -> Option<()> {
        let rust_name = element.name.to_string();
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
        // Emitted after children: `dynamic_attr` is deferred to `close_element` in the op tape,
        // and its slot index must follow the children's dynamic nodes.
        let id = self.dynamic_attributes.len();
        self.template.dynamic_attr();
        self.dynamic_slots.push(HotReloadDynamicSlot::Attribute(id));
        self.dynamic_attributes.push(attr);
        Some(())
    }

    fn static_text(&mut self, text: &'a TextNode) -> Option<()> {
        let value = text.input.to_static()?;
        self.template.static_text(intern(value.as_str()));
        Some(())
    }

    fn dynamic_node(&mut self, node: &'a BodyNode, following_static_at_parent: bool) -> Option<()> {
        let id = self.dynamic_nodes.len();
        self.template.dynamic_node(following_static_at_parent);
        self.dynamic_slots.push(HotReloadDynamicSlot::Node(id));
        self.dynamic_nodes.push(node);
        Some(())
    }
}
