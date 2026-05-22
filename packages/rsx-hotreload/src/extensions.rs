use dioxus_core::TemplateNode;
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

pub fn to_template_attribute<Ctx: HotReloadingContext>(
    attr: &Attribute,
) -> dioxus_core::TemplateAttribute {
    use dioxus_core::TemplateAttribute;

    // If it's a dynamic node, just return it
    // For dynamic attributes, we need to check the mapping to see if that mapping exists
    // todo: one day we could generate new dynamic attributes on the fly if they're a literal,
    // or something sufficiently serializable
    //  (ie `checked`` being a bool and bools being interpretable)
    //
    // For now, just give up if that attribute doesn't exist in the mapping
    if !attr.is_static_str_literal() {
        let id = attr.dyn_idx.get();
        return TemplateAttribute::Dynamic { id };
    }

    // Otherwise it's a static node and we can build it
    let (_, value) = attr.as_static_str_literal().unwrap();
    let (name, namespace) = html_tag_and_namespace::<Ctx>(attr);

    TemplateAttribute::Static {
        name,
        namespace,
        value: intern(value.to_static().unwrap().as_str()),
    }
}

/// Convert this BodyNode into a TemplateNode.
///
/// dioxus-core uses this to understand templates at compiletime
pub fn to_template_node<Ctx: HotReloadingContext>(node: &BodyNode) -> dioxus_core::TemplateNode {
    use dioxus_core::TemplateNode;
    match node {
        BodyNode::Element(el) => {
            let rust_name = el.name.to_string();

            let (tag, namespace) =
                Ctx::map_element(&rust_name).unwrap_or((intern(rust_name.as_str()), None));

            TemplateNode::Element {
                tag,
                namespace,
                children: intern(
                    el.children
                        .iter()
                        .map(|c| to_template_node::<Ctx>(c))
                        .collect::<Vec<_>>(),
                ),
                attrs: intern(
                    el.merged_attributes
                        .iter()
                        .map(|attr| to_template_attribute::<Ctx>(attr))
                        .collect::<Vec<_>>(),
                ),
            }
        }
        BodyNode::Text(text) => text_to_template_node(text),
        BodyNode::RawExpr(exp) => TemplateNode::Dynamic {
            id: exp.dyn_idx.get(),
        },
        BodyNode::Component(comp) => TemplateNode::Dynamic {
            id: comp.dyn_idx.get(),
        },
        BodyNode::ForLoop(floop) => TemplateNode::Dynamic {
            id: floop.dyn_idx.get(),
        },
        BodyNode::IfChain(chain) => TemplateNode::Dynamic {
            id: chain.dyn_idx.get(),
        },
    }
}
pub fn text_to_template_node(node: &TextNode) -> TemplateNode {
    match node.input.to_static() {
        Some(text) => TemplateNode::Text {
            text: intern(text.as_str()),
        },
        None => TemplateNode::Dynamic {
            id: node.dyn_idx.get(),
        },
    }
}
