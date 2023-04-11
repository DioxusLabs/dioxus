use dioxus_core::{Template, TemplateAttribute, TemplateNode};
use std::fmt::Write;

/// Render a template to an HTML string
///
/// Useful for sending over the wire. Can be used to with innerHtml to create templates with little work
pub fn render_template_to_html(template: &Template) -> String {
    let mut out = String::new();

    for root in template.roots {
        render_template_node(root, &mut out).unwrap();
    }

    out
}

fn render_template_node(node: &TemplateNode, out: &mut String) -> std::fmt::Result {
    match node {
        TemplateNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            write!(out, "<{tag}")?;
            for attr in *attrs {
                if let TemplateAttribute::Static { name, value, .. } = attr {
                    write!(out, "{name}=\"{value}\"")?;
                }
            }
            for child in *children {
                render_template_node(child, out)?;
            }
            write!(out, "</{tag}>")?;
        }
        TemplateNode::Text { text: t } => write!(out, "{t}")?,
        TemplateNode::Dynamic { id: _ } => write!(out, "<pre hidden />")?,
        TemplateNode::DynamicText { id: t } => write!(out, "<!-- --> {t} <!-- -->")?,
    };
    Ok(())
}
