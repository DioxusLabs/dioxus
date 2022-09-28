use crate::Buffer;
use dioxus_rsx::*;
use proc_macro2::Span;
use std::{fmt::Result, fmt::Write};
use syn::{spanned::Spanned, Expr};

#[derive(Debug)]
enum ShortOptimization {
    // Special because we want to print the closing bracket immediately
    Empty,

    // Special optimization to put everything on the same line
    Oneliner,

    // Optimization where children flow but props remain fixed on top
    PropsOnTop,

    // The noisiest optimization where everything flows
    NoOpt,
}

impl Buffer {
    pub fn write_element(
        &mut self,
        Element {
            name,
            key,
            attributes,
            children,
            _is_static,
        }: &Element,
    ) -> Result {
        /*
            1. Write the tag
            2. Write the key
            3. Write the attributes
            4. Write the children
        */

        write!(self.buf, "{name} {{")?;

        // decide if we have any special optimizations
        // Default with none, opt the cases in one-by-one
        let mut opt_level = ShortOptimization::NoOpt;

        // check if we have a lot of attributes
        let attr_len = self.is_short_attrs(attributes);
        let is_short_attr_list = attr_len < 80;
        let is_small_children = self.is_short_children(children).is_some();

        // if we have few attributes and a lot of children, place the attrs on top
        if is_short_attr_list && !is_small_children {
            opt_level = ShortOptimization::PropsOnTop;
        }

        // even if the attr is long, it should be put on one line
        if !is_short_attr_list && attributes.len() <= 1 {
            if children.is_empty() {
                opt_level = ShortOptimization::Oneliner;
            } else {
                opt_level = ShortOptimization::PropsOnTop;
            }
        }

        // if we have few children and few attributes, make it a one-liner
        if is_short_attr_list && is_small_children {
            opt_level = ShortOptimization::Oneliner;
        }

        // If there's nothing at all, empty optimization
        if attributes.is_empty() && children.is_empty() && key.is_none() {
            opt_level = ShortOptimization::Empty;
        }

        // multiline handlers bump everything down
        if attr_len > 1000 {
            opt_level = ShortOptimization::NoOpt;
        }

        match opt_level {
            ShortOptimization::Empty => {}
            ShortOptimization::Oneliner => {
                write!(self.buf, " ")?;

                self.write_attributes(attributes, key, true)?;

                if !children.is_empty() && !attributes.is_empty() {
                    write!(self.buf, ", ")?;
                }

                for (id, child) in children.iter().enumerate() {
                    self.write_ident(child)?;
                    if id != children.len() - 1 && children.len() > 1 {
                        write!(self.buf, ", ")?;
                    }
                }

                write!(self.buf, " ")?;
            }

            ShortOptimization::PropsOnTop => {
                if !attributes.is_empty() || key.is_some() {
                    write!(self.buf, " ")?;
                }
                self.write_attributes(attributes, key, true)?;

                if !children.is_empty() && !attributes.is_empty() {
                    write!(self.buf, ",")?;
                }

                if !children.is_empty() {
                    self.write_body_indented(children)?;
                }
                self.tabbed_line()?;
            }

            ShortOptimization::NoOpt => {
                self.write_attributes(attributes, key, false)?;

                if !children.is_empty() && !attributes.is_empty() {
                    write!(self.buf, ",")?;
                }

                if !children.is_empty() {
                    self.write_body_indented(children)?;
                }
                self.tabbed_line()?;
            }
        }

        write!(self.buf, "}}")?;

        Ok(())
    }

    fn write_attributes(
        &mut self,
        attributes: &[ElementAttrNamed],
        key: &Option<syn::LitStr>,
        sameline: bool,
    ) -> Result {
        let mut attr_iter = attributes.iter().peekable();

        if let Some(key) = key {
            if !sameline {
                self.indented_tabbed_line()?;
            }
            write!(self.buf, "key: \"{}\"", key.value())?;
            if !attributes.is_empty() {
                write!(self.buf, ",")?;
                if sameline {
                    write!(self.buf, " ")?;
                }
            }
        }

        while let Some(attr) = attr_iter.next() {
            self.indent += 1;
            if !sameline {
                self.write_comments(attr.attr.flart())?;
            }
            self.indent -= 1;

            if !sameline {
                self.indented_tabbed_line()?;
            }

            self.write_attribute(attr)?;

            if attr_iter.peek().is_some() {
                write!(self.buf, ",")?;

                if sameline {
                    write!(self.buf, " ")?;
                }
            }
        }

        Ok(())
    }

    fn write_attribute(&mut self, attr: &ElementAttrNamed) -> Result {
        match &attr.attr {
            ElementAttr::AttrText { name, value } => {
                write!(self.buf, "{name}: \"{value}\"", value = value.value())?;
            }
            ElementAttr::AttrExpression { name, value } => {
                let out = prettyplease::unparse_expr(value);
                write!(self.buf, "{}: {}", name, out)?;
            }

            ElementAttr::CustomAttrText { name, value } => {
                write!(
                    self.buf,
                    "\"{name}\": \"{value}\"",
                    name = name.value(),
                    value = value.value()
                )?;
            }

            ElementAttr::CustomAttrExpression { name, value } => {
                let out = prettyplease::unparse_expr(value);
                write!(self.buf, "\"{}\": {}", name.value(), out)?;
            }

            ElementAttr::EventTokens { name, tokens } => {
                let out = self.retrieve_formatted_expr(tokens).to_string();

                let mut lines = out.split('\n').peekable();
                let first = lines.next().unwrap();

                // a one-liner for whatever reason
                // Does not need a new line
                if lines.peek().is_none() {
                    write!(self.buf, "{}: {}", name, first)?;
                } else {
                    writeln!(self.buf, "{}: {}", name, first)?;

                    while let Some(line) = lines.next() {
                        self.indented_tab()?;
                        write!(self.buf, "{}", line)?;
                        if lines.peek().is_none() {
                            write!(self.buf, "")?;
                        } else {
                            writeln!(self.buf)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // make sure the comments are actually relevant to this element.
    // test by making sure this element is the primary element on this line
    pub fn current_span_is_primary(&self, location: Span) -> bool {
        let start = location.start();
        let line_start = start.line - 1;

        let this_line = self.src[line_start].as_str();

        let beginning = if this_line.len() > start.column {
            this_line[..start.column].trim()
        } else {
            ""
        };

        // dbg!(beginning);

        beginning.is_empty()
    }

    // check if the children are short enough to be on the same line
    // We don't have the notion of current line depth - each line tries to be < 80 total
    // returns the total line length if it's short
    // returns none if the length exceeds the limit
    // I think this eventually becomes quadratic :(
    pub fn is_short_children(&mut self, children: &[BodyNode]) -> Option<usize> {
        if children.is_empty() {
            // todo: allow elements with comments but no children
            // like div { /* comment */ }
            return Some(0);
        }

        for child in children {
            if self.current_span_is_primary(child.span()) {
                'line: for line in self.src[..child.span().start().line - 1].iter().rev() {
                    match (line.trim().starts_with("//"), line.is_empty()) {
                        (true, _) => return None,
                        (_, true) => continue 'line,
                        _ => break 'line,
                    }
                }
            }
        }

        match children {
            [BodyNode::Text(ref text)] => Some(text.value().len()),
            [BodyNode::Component(ref comp)] => {
                let attr_len = self.field_len(&comp.fields, &comp.manual_props);

                if attr_len > 80 {
                    None
                } else {
                    self.is_short_children(&comp.children)
                        .map(|child_len| child_len + attr_len)
                }
            }
            [BodyNode::RawExpr(ref expr)] => {
                // TODO: let rawexprs to be inlined
                get_expr_length(expr)
            }
            [BodyNode::Element(ref el)] => {
                let attr_len = self.is_short_attrs(&el.attributes);

                if el.children.is_empty() && attr_len < 80 {
                    return Some(el.name.to_string().len());
                }

                if el.children.len() == 1 {
                    if let BodyNode::Text(ref text) = el.children[0] {
                        if text.value().len() + el.name.to_string().len() + attr_len < 80 {
                            return Some(text.value().len() + el.name.to_string().len() + attr_len);
                        }
                    }
                }

                None
            }
            // todo, allow non-elements to be on the same line
            items => {
                let mut total_count = 0;

                for item in items {
                    match item {
                        BodyNode::Component(_) | BodyNode::Element(_) => return None,
                        BodyNode::Text(text) => total_count += text.value().len(),
                        BodyNode::RawExpr(expr) => match get_expr_length(expr) {
                            Some(len) => total_count += len,
                            None => return None,
                        },
                    }
                }

                Some(total_count)
            }
        }
    }
}

fn get_expr_length(expr: &Expr) -> Option<usize> {
    let span = expr.span();
    let (start, end) = (span.start(), span.end());
    if start.line == end.line {
        Some(end.column - start.column)
    } else {
        None
    }
}
