pub(super) mod dom;
pub(crate) mod vdom;

use std::fmt::Write;

use dioxus_autofmt::write_block_out;
use dioxus_core::{Attribute, AttributeValue, TemplateAttribute};
use dioxus_rsx::{
    Attribute as RsxAttribute, AttributeName as RsxAttributeName,
    AttributeValue as RsxAttributeValue, BodyNode, CallBody, Component as RsxComponent,
    Diagnostics, Element as RsxElement, ElementName as RsxElementName, ExprNode, HotLiteral,
    PartialExpr, TemplateBody, TextNode,
};
use syn::parse_quote;

pub(crate) use self::vdom::serialize_vnode_subtree;

pub(super) const STYLE_NAMESPACE: &str = "style";
pub(super) const DANGEROUS_INNER_HTML_ATTRIBUTE: &str = "dangerous_inner_html";

pub(super) fn append_style_declaration(into: &mut String, name: &str, value: &str) {
    let _ = write!(into, "{name}:{value};");
}

pub(super) fn append_dynamic_style_declaration(
    into: &mut String,
    name: &str,
    value: &AttributeValue,
) {
    let _ = write!(into, "{name}:");
    match value {
        AttributeValue::Text(value) => into.push_str(value),
        AttributeValue::Float(value) => {
            let _ = write!(into, "{value}");
        }
        AttributeValue::Int(value) => {
            let _ = write!(into, "{value}");
        }
        AttributeValue::Bool(value) => {
            let _ = write!(into, "{value}");
        }
        AttributeValue::Any(_) | AttributeValue::Listener(_) | AttributeValue::None => {}
    }
    into.push(';');
}

/// A non-style attribute yielded by [`walk_template_attrs`].
pub(super) enum TemplateAttrRef<'a> {
    Static {
        name: &'static str,
        value: &'static str,
        namespace: Option<&'static str>,
    },
    Dynamic(&'a Attribute),
}

/// Walk a template's static attrs plus resolved dynamic attribute groups,
/// folding any `style`-namespaced entries into a single declaration string and
/// invoking the visitor for everything else. Returns the combined style value
/// if any style fragments were collected.
///
/// Both the hydration-validation attribute comparator and the RSX serializer
/// need the exact same traversal + style-folding; this is the shared seam.
pub(super) fn walk_template_attrs<'a>(
    static_attrs: &'static [TemplateAttribute],
    dynamic_groups: impl IntoIterator<Item = &'a [Attribute]>,
    mut visit: impl FnMut(TemplateAttrRef<'a>),
) -> Option<String> {
    let mut static_styles = String::new();
    let mut dynamic_styles = String::new();

    for attr in static_attrs {
        let TemplateAttribute::Static {
            name,
            value,
            namespace,
        } = attr
        else {
            continue;
        };
        if *namespace == Some(STYLE_NAMESPACE) {
            append_style_declaration(&mut static_styles, name, value);
        } else {
            visit(TemplateAttrRef::Static {
                name,
                value,
                namespace: *namespace,
            });
        }
    }

    for group in dynamic_groups {
        for attr in group {
            if attr.namespace == Some(STYLE_NAMESPACE) {
                append_dynamic_style_declaration(&mut dynamic_styles, attr.name, &attr.value);
            } else {
                visit(TemplateAttrRef::Dynamic(attr));
            }
        }
    }

    if static_styles.is_empty() && dynamic_styles.is_empty() {
        None
    } else {
        Some(format!("{static_styles}{dynamic_styles}"))
    }
}

pub(super) fn missing_node() -> BodyNode {
    component_node("missing_node")
}

fn component_node(name: &str) -> BodyNode {
    match syn::parse_str::<syn::Path>(name) {
        Ok(path) => BodyNode::Component(RsxComponent {
            name: path,
            generics: None,
            fields: Vec::new(),
            component_literal_dyn_idx: Vec::new(),
            spreads: Vec::new(),
            brace: Some(Default::default()),
            children: TemplateBody::new(Vec::new()),
            dyn_idx: Default::default(),
            diagnostics: Diagnostics::new(),
        }),
        Err(_) => BodyNode::Text(TextNode::from_text(&format!("<component {name}>"))),
    }
}

pub(super) fn placeholder_node() -> BodyNode {
    BodyNode::RawExpr(ExprNode {
        expr: PartialExpr::from_expr(&parse_quote!(VNode::placeholder())),
        dyn_idx: Default::default(),
    })
}

fn element_node(tag: &str, mut attributes: Vec<RsxAttribute>, children: Vec<BodyNode>) -> BodyNode {
    attributes.sort_by_key(|attr| attr.name.to_string());
    BodyNode::Element(RsxElement {
        name: syn::parse_str(tag).unwrap_or_else(|_| RsxElementName::Custom(parse_quote!(#tag))),
        raw_attributes: attributes.clone(),
        merged_attributes: attributes,
        spreads: Vec::new(),
        children,
        brace: Some(Default::default()),
        diagnostics: Diagnostics::new(),
    })
}

fn text_attribute(name: &str, value: &str) -> RsxAttribute {
    RsxAttribute::from_raw(
        rsx_attribute_name(name),
        RsxAttributeValue::AttrLiteral(HotLiteral::from_raw_text(value)),
    )
}

fn expr_attribute(name: &str, value: syn::Expr) -> RsxAttribute {
    RsxAttribute::from_raw(
        rsx_attribute_name(name),
        RsxAttributeValue::AttrExpr(PartialExpr::from_expr(&value)),
    )
}

pub(super) fn format_rsx_nodes(nodes: Vec<BodyNode>) -> String {
    let nodes = if nodes.is_empty() {
        vec![missing_node()]
    } else {
        nodes
    };

    let body = CallBody::new(TemplateBody::new(nodes));
    write_block_out(&body)
        .map(normalize_formatted_rsx)
        .unwrap_or_else(|| format!("{body:?}"))
}

pub(super) fn normalize_formatted_rsx(formatted: String) -> String {
    if formatted.trim().is_empty() {
        return String::new();
    }

    let shared_indent = formatted
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|ch| ch.is_whitespace())
                .collect::<String>()
        })
        .reduce(|left, right| {
            left.chars()
                .zip(right.chars())
                .take_while(|(left, right)| left == right)
                .map(|(ch, _)| ch)
                .collect()
        })
        .unwrap_or_default();

    let dedented = if shared_indent.is_empty() {
        formatted
    } else {
        formatted
            .lines()
            .map(|line| line.strip_prefix(&shared_indent).unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n")
    };

    dedented.trim().to_string()
}

fn rsx_attribute_name(name: &str) -> RsxAttributeName {
    syn::parse_str(name)
        .map(RsxAttributeName::BuiltIn)
        .unwrap_or_else(|_| RsxAttributeName::Custom(parse_quote!(#name)))
}

pub(super) fn is_internal_attribute_name(name: &str) -> bool {
    name.starts_with("on") || name.starts_with("data-node") || name.starts_with("data-dioxus")
}
