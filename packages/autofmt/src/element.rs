use crate::{ifmt_to_string, Writer};
use dioxus_rsx::*;
use proc_macro2::Span;
use quote::ToTokens;
use std::{
    fmt::Result,
    fmt::{self, Write},
};
use syn::{spanned::Spanned, token::Brace, Expr};

#[derive(Debug)]
enum ShortOptimization {
    /// Special because we want to print the closing bracket immediately
    ///
    /// IE
    /// `div {}` instead of `div { }`
    Empty,

    /// Special optimization to put everything on the same line and add some buffer spaces
    ///
    /// IE
    ///
    /// `div { "asdasd" }` instead of a multiline variant
    Oneliner,

    /// Optimization where children flow but props remain fixed on top
    PropsOnTop,

    /// The noisiest optimization where everything flows
    NoOpt,
}

/*
// whitespace
div {
    // some whitespace
    class: "asdasd"

    // whjiot
    asdasd // whitespace
}
*/

impl Writer<'_> {
    pub fn write_element(&mut self, el: &Element) -> Result {
        let Element {
            name,
            key,
            attributes,
            children,
            brace,
            ..
        } = el;

        /*
            1. Write the tag
            2. Write the key
            3. Write the attributes
            4. Write the children
        */

        write!(self.out, "{name} {{")?;

        // decide if we have any special optimizations
        // Default with none, opt the cases in one-by-one
        let mut opt_level = ShortOptimization::NoOpt;

        // check if we have a lot of attributes
        let attr_len = self.is_short_attrs(attributes);
        let is_short_attr_list = (attr_len + self.out.indent_level * 4) < 80;
        let children_len = self.is_short_children(children);
        let is_small_children = children_len.is_some();

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
            if children_len.unwrap() + attr_len + self.out.indent_level * 4 < 100 {
                opt_level = ShortOptimization::Oneliner;
            } else {
                opt_level = ShortOptimization::PropsOnTop;
            }
        }

        // If there's nothing at all, empty optimization
        if attributes.is_empty() && children.is_empty() && key.is_none() {
            opt_level = ShortOptimization::Empty;

            // Write comments if they exist
            self.write_todo_body(brace)?;
        }

        // multiline handlers bump everything down
        if attr_len > 1000 || self.out.indent.split_line_attributes() {
            opt_level = ShortOptimization::NoOpt;
        }

        match opt_level {
            ShortOptimization::Empty => {}
            ShortOptimization::Oneliner => {
                write!(self.out, " ")?;

                self.write_attributes(attributes, key, true)?;

                if !children.is_empty() && (!attributes.is_empty() || key.is_some()) {
                    write!(self.out, ", ")?;
                }

                for (id, child) in children.iter().enumerate() {
                    self.write_ident(child)?;
                    if id != children.len() - 1 && children.len() > 1 {
                        write!(self.out, ", ")?;
                    }
                }

                write!(self.out, " ")?;
            }

            ShortOptimization::PropsOnTop => {
                if !attributes.is_empty() || key.is_some() {
                    write!(self.out, " ")?;
                }
                self.write_attributes(attributes, key, true)?;

                if !children.is_empty() && (!attributes.is_empty() || key.is_some()) {
                    write!(self.out, ",")?;
                }

                if !children.is_empty() {
                    self.write_body_indented(children)?;
                }
                self.out.tabbed_line()?;
            }

            ShortOptimization::NoOpt => {
                self.write_attributes(attributes, key, false)?;

                if !children.is_empty() && (!attributes.is_empty() || key.is_some()) {
                    write!(self.out, ",")?;
                }

                if !children.is_empty() {
                    self.write_body_indented(children)?;
                }

                self.out.tabbed_line()?;
            }
        }

        write!(self.out, "}}")?;

        Ok(())
    }

    fn write_attributes(
        &mut self,
        attributes: &[AttributeType],
        key: &Option<IfmtInput>,
        sameline: bool,
    ) -> Result {
        let mut attr_iter = attributes.iter().peekable();

        if let Some(key) = key {
            if !sameline {
                self.out.indented_tabbed_line()?;
            }
            write!(self.out, "key: {}", ifmt_to_string(key))?;
            if !attributes.is_empty() {
                write!(self.out, ",")?;
                if sameline {
                    write!(self.out, " ")?;
                }
            }
        }

        while let Some(attr) = attr_iter.next() {
            self.out.indent_level += 1;
            if !sameline {
                self.write_comments(attr.start())?;
            }
            self.out.indent_level -= 1;

            if !sameline {
                self.out.indented_tabbed_line()?;
            }

            self.write_attribute(attr)?;

            if attr_iter.peek().is_some() {
                write!(self.out, ",")?;

                if sameline {
                    write!(self.out, " ")?;
                }
            }
        }

        Ok(())
    }

    fn write_attribute_name(&mut self, attr: &ElementAttrName) -> Result {
        match attr {
            ElementAttrName::BuiltIn(name) => {
                write!(self.out, "{}", name)?;
            }
            ElementAttrName::Custom(name) => {
                write!(self.out, "{}", name.to_token_stream())?;
            }
        }

        Ok(())
    }

    fn write_attribute_value(&mut self, value: &ElementAttrValue) -> Result {
        match value {
            ElementAttrValue::AttrOptionalExpr { condition, value } => {
                write!(
                    self.out,
                    "if {condition} {{ ",
                    condition = prettyplease::unparse_expr(condition),
                )?;
                self.write_attribute_value(value)?;
                write!(self.out, " }}")?;
            }
            ElementAttrValue::AttrLiteral(value) => {
                write!(self.out, "{value}", value = ifmt_to_string(value))?;
            }
            ElementAttrValue::Shorthand(value) => {
                write!(self.out, "{value}",)?;
            }
            ElementAttrValue::AttrExpr(value) => {
                let out = prettyplease::unparse_expr(value);
                let mut lines = out.split('\n').peekable();
                let first = lines.next().unwrap();

                // a one-liner for whatever reason
                // Does not need a new line
                if lines.peek().is_none() {
                    write!(self.out, "{first}")?;
                } else {
                    writeln!(self.out, "{first}")?;

                    while let Some(line) = lines.next() {
                        self.out.indented_tab()?;
                        write!(self.out, "{line}")?;
                        if lines.peek().is_none() {
                            write!(self.out, "")?;
                        } else {
                            writeln!(self.out)?;
                        }
                    }
                }
            }
            ElementAttrValue::EventTokens(tokens) => {
                let out = self.retrieve_formatted_expr(tokens).to_string();
                let mut lines = out.split('\n').peekable();
                let first = lines.next().unwrap();

                // a one-liner for whatever reason
                // Does not need a new line
                if lines.peek().is_none() {
                    write!(self.out, "{first}")?;
                } else {
                    writeln!(self.out, "{first}")?;

                    while let Some(line) = lines.next() {
                        self.out.indented_tab()?;
                        write!(self.out, "{line}")?;
                        if lines.peek().is_none() {
                            write!(self.out, "")?;
                        } else {
                            writeln!(self.out)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn write_attribute(&mut self, attr: &AttributeType) -> Result {
        match attr {
            AttributeType::Named(attr) => self.write_named_attribute(attr),
            AttributeType::Spread(attr) => self.write_spread_attribute(attr),
        }
    }

    fn write_named_attribute(&mut self, attr: &ElementAttrNamed) -> Result {
        self.write_attribute_name(&attr.attr.name)?;
        write!(self.out, ": ")?;
        self.write_attribute_value(&attr.attr.value)?;

        Ok(())
    }

    fn write_spread_attribute(&mut self, attr: &Expr) -> Result {
        write!(self.out, "..")?;
        write!(self.out, "{}", prettyplease::unparse_expr(attr))?;

        Ok(())
    }

    // make sure the comments are actually relevant to this element.
    // test by making sure this element is the primary element on this line
    pub fn current_span_is_primary(&self, location: Span) -> bool {
        let start = location.start();
        let line_start = start.line - 1;

        let beginning = self
            .src
            .get(line_start)
            .filter(|this_line| this_line.len() > start.column)
            .map(|this_line| this_line[..start.column].trim())
            .unwrap_or_default();

        beginning.is_empty()
    }

    pub fn is_empty_children(&self, children: &[BodyNode]) -> bool {
        children.is_empty()
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
            // or
            // div {
            //  // some helpful
            // }
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
            [BodyNode::Text(ref text)] => Some(ifmt_to_string(text).len()),
            [BodyNode::Component(ref comp)] => {
                let attr_len = self.field_len(&comp.fields, &comp.manual_props);

                if attr_len > 80 {
                    None
                } else if comp.children.is_empty() {
                    Some(attr_len)
                } else {
                    None
                }
            }
            // TODO: let rawexprs to be inlined
            [BodyNode::RawExpr(ref expr)] => get_expr_length(expr),
            [BodyNode::Element(ref el)] => {
                let attr_len = self.is_short_attrs(&el.attributes);

                if el.children.is_empty() && attr_len < 80 {
                    return Some(el.name.to_string().len());
                }

                if el.children.len() == 1 {
                    if let BodyNode::Text(ref text) = el.children[0] {
                        let value = ifmt_to_string(text);

                        if value.len() + el.name.to_string().len() + attr_len < 80 {
                            return Some(value.len() + el.name.to_string().len() + attr_len);
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
                        BodyNode::Text(text) => {
                            total_count += ifmt_to_string(text).len();
                        }
                        BodyNode::RawExpr(expr) => match get_expr_length(expr) {
                            Some(len) => total_count += len,
                            None => return None,
                        },
                        BodyNode::ForLoop(_forloop) => return None,
                        BodyNode::IfChain(_chain) => return None,
                    }
                }

                Some(total_count)
            }
        }
    }

    /// empty everything except for some comments
    fn write_todo_body(&mut self, brace: &Brace) -> fmt::Result {
        let span = brace.span.span();
        let start = span.start();
        let end = span.end();

        if start.line == end.line {
            return Ok(());
        }

        writeln!(self.out)?;

        for idx in start.line..end.line {
            let line = &self.src[idx];
            if line.trim().starts_with("//") {
                for _ in 0..self.out.indent_level + 1 {
                    write!(self.out, "    ")?
                }
                writeln!(self.out, "{}", line.trim()).unwrap();
            }
        }

        for _ in 0..self.out.indent_level {
            write!(self.out, "    ")?
        }

        Ok(())
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
