use crate::{
    component::write_component, element::write_element, expr::write_raw_expr, util::write_tabs,
};
use dioxus_rsx::*;
use std::fmt::{self, Write};

pub fn write_ident(
    buf: &mut String,
    lines: &[&str],
    node: &BodyNode,
    indent: usize,
) -> fmt::Result {
    match node {
        BodyNode::Element(el) => write_element(el, buf, lines, indent),
        BodyNode::Component(component) => write_component(component, buf, indent, lines),
        BodyNode::Text(text) => write_text(text, buf, indent),
        BodyNode::RawExpr(exp) => write_raw_expr(exp, indent, lines, buf),
    }
}

fn write_text(text: &syn::LitStr, buf: &mut String, indent: usize) -> fmt::Result {
    write!(buf, "\"{}\"", text.value())
}
