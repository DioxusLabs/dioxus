use crate::{util::*, write_ident};
use dioxus_rsx::*;
use std::{fmt, fmt::Write};

enum ShortOptimization {
    // Special because we want to print the closing bracket immediately
    Empty,
    Oneliner,
    PropsOnTop,
    NoOpt,
}

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
        1. Write the tag
        2. Write the key
        3. Write the attributes
        4. Write the children
    */

    write!(buf, "{name} {{")?;

    // decide if we have any special optimizations
    // Default with none, opt the cases in one-by-one
    let mut opt_level = ShortOptimization::NoOpt;

    // check if we have a lot of attributes
    let is_short_attr_list = is_short_attrs(attributes);
    let is_small_children = is_short_children(children);

    // if we have few attributes and a lot of children, place the attrs on top
    if is_short_attr_list && !is_small_children {
        opt_level = ShortOptimization::PropsOnTop;
    }

    // if we have few children and few attributes, make it a one-liner
    if is_short_attr_list && is_small_children {
        opt_level = ShortOptimization::Oneliner;
    }

    // If there's nothing at all, empty optimization
    if attributes.is_empty() && children.is_empty() && key.is_none() {
        opt_level = ShortOptimization::Empty;
    }

    match opt_level {
        ShortOptimization::Empty => write!(buf, "}}")?,
        ShortOptimization::Oneliner => {
            write!(buf, " ")?;
            write_attributes(buf, attributes, true, indent)?;

            if !children.is_empty() && !attributes.is_empty() {
                write!(buf, ", ")?;
            }

            // write the children
            for child in children {
                write_ident(buf, lines, child, indent + 1)?;
            }

            write!(buf, " }}")?;
        }

        ShortOptimization::PropsOnTop => {
            write!(buf, " ")?;
            write_attributes(buf, attributes, true, indent)?;

            if !children.is_empty() && !attributes.is_empty() {
                write!(buf, ",")?;
            }

            // write the children
            for child in children {
                writeln!(buf)?;
                write_tabs(buf, indent + 1)?;
                write_ident(buf, lines, child, indent + 1)?;
            }

            writeln!(buf)?;
            write_tabs(buf, indent)?;
            write!(buf, "}}")?;
        }

        ShortOptimization::NoOpt => {
            // write the key

            // write the attributes
            write_attributes(buf, attributes, false, indent)?;

            // write the children
            for child in children {
                writeln!(buf)?;
                write_tabs(buf, indent + 1)?;
                write_ident(buf, lines, child, indent + 1)?;
            }

            writeln!(buf)?;
            write_tabs(buf, indent)?;
            write!(buf, "}}")?;
        }
    }

    Ok(())
}

fn is_short_attrs(attrs: &[ElementAttrNamed]) -> bool {
    let total_attr_len = extract_attr_len(attrs);
    total_attr_len < 80
}

// check if the children are short enough to be on the same line
// We don't have the notion of current line depth - each line tries to be < 80 total
fn is_short_children(children: &[BodyNode]) -> bool {
    if children.is_empty() {
        return true;
    }

    if children.len() == 1 {
        if let BodyNode::Text(ref text) = &children[0] {
            return text.value().len() < 80;
        }
    }

    false
}

fn write_key() {
    // if let Some(key) = key.as_ref().map(|f| f.value()) {
    //     if is_long_attr_list {
    //         writeln!(buf)?;
    //         write_tabs(buf, indent + 1)?;
    //     } else {
    //         write!(buf, " ")?;
    //     }
    //     write!(buf, "key: \"{key}\"")?;

    //     if !attributes.is_empty() {
    //         write!(buf, ",")?;
    //     }
    // }
}

fn write_attributes(
    buf: &mut String,
    attributes: &[ElementAttrNamed],
    sameline: bool,
    indent: usize,
) -> fmt::Result {
    let mut attr_iter = attributes.iter().peekable();

    while let Some(attr) = attr_iter.next() {
        write_attribute(buf, attr, indent)?;

        if attr_iter.peek().is_some() {
            write!(buf, ",")?;

            if sameline {
                write!(buf, " ")?;
            } else {
                writeln!(buf)?;
                write_tabs(buf, indent + 1)?;
            }
        }
    }

    Ok(())
}

fn write_attribute(buf: &mut String, attr: &ElementAttrNamed, indent: usize) -> fmt::Result {
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

    Ok(())
}
