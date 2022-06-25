use crate::{util::*, write_ident};
use dioxus_rsx::*;
use std::{fmt, fmt::Write};

pub fn write_element(
    Element {
        name,
        key,
        attributes,
        children,
        _is_static,
    }: &Element,
    buf: &mut String,
    lines: &[&str],
    node: &BodyNode,
    indent: usize,
) -> fmt::Result {
    /*
    Write the tag
    Write the key
    Write the attributes
    Write the children
    */

    write!(buf, "{name} {{")?;

    let total_attr_len = extract_attr_len(attributes);
    let is_long_attr_list = total_attr_len > 80;

    if let Some(key) = key.as_ref().map(|f| f.value()) {
        if is_long_attr_list {
            writeln!(buf)?;
            write_tabs(buf, indent + 1)?;
        } else {
            write!(buf, " ")?;
        }
        write!(buf, "key: \"{key}\"")?;

        if !attributes.is_empty() {
            write!(buf, ",")?;
        }
    }

    let mut attr_iter = attributes.iter().peekable();

    while let Some(attr) = attr_iter.next() {
        if is_long_attr_list {
            writeln!(buf)?;
            write_tabs(buf, indent + 1)?;
        } else {
            write!(buf, " ")?;
        }

        match &attr.attr {
            ElementAttr::AttrText { name, value } => {
                write!(buf, "{name}: \"{value}\"", value = value.value())?;
            }
            ElementAttr::AttrExpression { name, value } => {
                let out = prettyplease::unparse_expr(value);
                write!(buf, "{}: {}", name, out)?;
            }

            ElementAttr::CustomAttrText { name, value } => {
                write!(
                    buf,
                    "\"{name}\": \"{value}\"",
                    name = name.value(),
                    value = value.value()
                )?;
            }

            ElementAttr::CustomAttrExpression { name, value } => {
                let out = prettyplease::unparse_expr(value);
                write!(buf, "\"{}\": {}", name.value(), out)?;
            }

            ElementAttr::EventTokens { name, tokens } => {
                let out = prettyplease::unparse_expr(tokens);

                let mut lines = out.split('\n').peekable();
                let first = lines.next().unwrap();

                // a one-liner for whatever reason
                // Does not need a new line
                if lines.peek().is_none() {
                    write!(buf, "{}: {}", name, first)?;
                } else {
                    writeln!(buf, "{}: {}", name, first)?;

                    while let Some(line) = lines.next() {
                        write_tabs(buf, indent + 1)?;
                        write!(buf, "{}", line)?;
                        if lines.peek().is_none() {
                            write!(buf, "")?;
                        } else {
                            writeln!(buf)?;
                        }
                    }
                }
            }
        }

        if attr_iter.peek().is_some() || !children.is_empty() {
            write!(buf, ",")?;
        }
    }

    // If the attr list is short, then we want to optimize for some cases
    let is_child_optimized = match children.as_slice() {
        // No children, just close the tag
        [] => true,

        // Only a text node, just write it out
        [BodyNode::Text(_)] => true,

        // If these have zero children and attributes, then we can just write out the tag
        [BodyNode::Component(ref comp)] => comp.body.is_empty() && comp.children.is_empty(),
        [BodyNode::Element(ref el)] => el.attributes.is_empty() && el.children.is_empty(),

        // Nothing else is optimized
        _ => false,
    };

    if !is_long_attr_list && is_child_optimized {
        write_ident(buf, lines, &children[0], indent)?;
    } else {
        for child in children {
            writeln!(buf)?;
            write_ident(buf, lines, child, indent + 1)?;
        }
    }

    // let text_val = text.value();
    // if total_attr_len + text_val.len() > 80 {
    //     writeln!(buf)?;
    //     write_tabs(buf, indent + 1)?;
    //     writeln!(buf, "\"{}\"", text.value())?;
    //     write_tabs(buf, indent)?;
    // } else {
    //     write!(buf, " \"{}\" ", text.value())?;
    // }

    // if is_long_attr_list {
    //     writeln!(buf)?;
    // }

    // for child in children {
    //     write_ident(buf, lines, child, indent + 1)?;
    // }

    // if is_long_attr_list {
    //     write_tabs(buf, indent)?;
    // } else {
    //     write!(buf, " ")?;
    // }

    writeln!(buf, "}}")?;

    Ok(())
}
