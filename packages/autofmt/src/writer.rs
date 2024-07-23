use crate::buffer::Buffer;
use crate::collect_macros::byte_offset;
use dioxus_rsx::{
    Attribute as AttributeType, AttributeName, AttributeValue as ElementAttrValue, BodyNode,
    Component, Element, ForLoop, IfChain, Spread, TemplateBody,
};
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use std::{
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};
use syn::{spanned::Spanned, token::Brace, Expr};

#[derive(Debug)]
pub struct Writer<'a> {
    pub raw_src: &'a str,
    pub src: Vec<&'a str>,
    pub cached_formats: HashMap<LineColumn, String>,
    pub comments: VecDeque<usize>,
    pub out: Buffer,
}

impl<'a> Writer<'a> {
    pub fn new(raw_src: &'a str) -> Self {
        let src = raw_src.lines().collect();
        Self {
            raw_src,
            src,
            cached_formats: HashMap::new(),
            comments: VecDeque::new(),
            out: Buffer::default(),
        }
    }

    pub fn consume(self) -> Option<String> {
        Some(self.out.buf)
    }

    pub fn write_rsx_call(&mut self, body: &TemplateBody) -> Result {
        match body.roots.len() {
            0 => {}
            1 if matches!(body.roots[0], BodyNode::Text(_)) => {
                write!(self.out, " ")?;
                self.write_ident(&body.roots[0])?;
                write!(self.out, " ")?;
            }
            _ => self.write_body_indented(&body.roots)?,
        }

        Ok(())
    }

    // Expects to be written directly into place
    pub fn write_ident(&mut self, node: &BodyNode) -> Result {
        match node {
            BodyNode::Element(el) => self.write_element(el),
            BodyNode::Component(component) => self.write_component(component),
            BodyNode::Text(text) => self.out.write_text(&text.input),
            BodyNode::RawExpr(exp) => self.write_raw_expr(exp.span()),
            BodyNode::ForLoop(forloop) => self.write_for_loop(forloop),
            BodyNode::IfChain(ifchain) => self.write_if_chain(ifchain),
        }
    }

    pub fn write_element(&mut self, el: &Element) -> Result {
        let Element {
            name,
            raw_attributes: attributes,
            children,
            spreads,
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

        let brace = brace.unwrap_or_default();
        self.write_rsx_block(attributes, spreads, children, &brace)?;

        write!(self.out, "}}")?;

        Ok(())
    }

    pub fn write_component(
        &mut self,
        Component {
            name,
            fields,
            children,
            generics,
            spreads,
            brace,
            ..
        }: &Component,
    ) -> Result {
        // Write the path by to_tokensing it and then removing all whitespace
        let mut name = name.to_token_stream().to_string();
        name.retain(|c| !c.is_whitespace());
        write!(self.out, "{name}")?;

        // Same idea with generics, write those via the to_tokens method and then remove all whitespace
        if let Some(generics) = generics {
            let mut written = generics.to_token_stream().to_string();
            written.retain(|c| !c.is_whitespace());
            write!(self.out, "{written}")?;
        }

        write!(self.out, " {{")?;

        self.write_rsx_block(fields, spreads, &children.roots, brace)?;

        write!(self.out, "}}")?;

        Ok(())
    }

    pub fn write_raw_expr(&mut self, placement: Span) -> Result {
        /*
        We want to normalize the expr to the appropriate indent level.
        */

        let start = placement.start();
        let end = placement.end();

        // if the expr is on one line, just write it directly
        if start.line == end.line {
            // split counting utf8 chars
            let start = byte_offset(self.raw_src, start);
            let end = byte_offset(self.raw_src, end);
            let row = self.raw_src[start..end].trim();
            write!(self.out, "{row}")?;
            return Ok(());
        }

        // If the expr is multiline, we want to collect all of its lines together and write them out properly
        // This involves unshifting the first line if it's aligned
        let first_line = &self.src[start.line - 1];
        write!(self.out, "{}", &first_line[start.column..].trim_start())?;

        let prev_block_indent_level = self.out.indent.count_indents(first_line);

        for (id, line) in self.src[start.line..end.line].iter().enumerate() {
            writeln!(self.out)?;

            // check if this is the last line
            let line = {
                if id == (end.line - start.line) - 1 {
                    &line[..end.column]
                } else {
                    line
                }
            };

            // trim the leading whitespace
            let previous_indent = self.out.indent.count_indents(line);
            let offset = previous_indent.saturating_sub(prev_block_indent_level);
            let required_indent = self.out.indent_level + offset;
            self.out.write_tabs(required_indent)?;

            let line = line.trim_start();
            write!(self.out, "{line}")?;
        }

        Ok(())
    }

    pub fn write_attr_comments(&mut self, brace: &Brace, attr_span: Span) -> Result {
        // There's a chance this line actually shares the same line as the previous
        // Only write comments if the comments actually belong to this line
        //
        // to do this, we check if the attr span starts on the same line as the brace
        // if it doesn't, we write the comments
        let brace_line = brace.span.span().start().line;
        let attr_line = attr_span.start().line;

        if brace_line != attr_line {
            self.write_comments(attr_span)?;
        }

        Ok(())
    }

    pub fn write_comments(&mut self, child: Span) -> Result {
        // collect all comments upwards
        // make sure we don't collect the comments of the node that we're currently under.

        let start = child.start();
        let line_start = start.line - 1;

        for (id, line) in self.src[..line_start].iter().enumerate().rev() {
            if line.trim().starts_with("//") || line.is_empty() {
                if id != 0 {
                    self.comments.push_front(id);
                }
            } else {
                break;
            }
        }

        let mut last_was_empty = false;
        while let Some(comment_line) = self.comments.pop_front() {
            let line = &self.src[comment_line];
            if line.is_empty() {
                if !last_was_empty {
                    self.out.new_line()?;
                }
                last_was_empty = true;
            } else {
                last_was_empty = false;
                self.out.tabbed_line()?;
                write!(self.out, "{}", self.src[comment_line].trim())?;
            }
        }

        Ok(())
    }

    // Push out the indent level and write each component, line by line
    pub fn write_body_indented(&mut self, children: &[BodyNode]) -> Result {
        self.out.indent_level += 1;

        self.write_body_no_indent(children)?;

        self.out.indent_level -= 1;
        Ok(())
    }

    pub fn write_body_no_indent(&mut self, children: &[BodyNode]) -> Result {
        for child in children {
            if self.current_span_is_primary(child.span()) {
                self.write_comments(child.span())?;
            };

            self.out.tabbed_line()?;
            self.write_ident(child)?;
        }

        Ok(())
    }

    pub(crate) fn attr_value_len(&mut self, value: &ElementAttrValue) -> usize {
        match value {
            ElementAttrValue::AttrOptionalExpr { condition, value } => {
                let condition_len = self.retrieve_formatted_expr(condition).len();
                let value_len = self.attr_value_len(value);
                condition_len + value_len + 6
            }
            ElementAttrValue::AttrLiteral(lit) => lit.to_string().len(),
            ElementAttrValue::Shorthand(expr) => expr.span().line_length(),
            ElementAttrValue::AttrExpr(expr) => expr
                .as_expr()
                .map(|expr| self.attr_expr_len(&expr))
                .unwrap_or(100000),
            ElementAttrValue::EventTokens(closure) => closure
                .as_expr()
                .map(|expr| self.attr_expr_len(&expr))
                .unwrap_or(100000),
        }
    }

    fn attr_expr_len(&mut self, expr: &Expr) -> usize {
        let out = self.retrieve_formatted_expr(expr);
        if out.contains('\n') {
            100000
        } else {
            out.len()
        }
    }

    pub(crate) fn is_short_attrs(
        &mut self,
        attributes: &[AttributeType],
        spreads: &[Spread],
    ) -> usize {
        let mut total = 0;

        // No more than 3 attributes before breaking the line
        if attributes.len() > 3 {
            return 100000;
        }

        for attr in attributes {
            if self.current_span_is_primary(attr.span()) {
                'line: for line in self.src[..attr.span().start().line - 1].iter().rev() {
                    match (line.trim().starts_with("//"), line.is_empty()) {
                        (true, _) => return 100000,
                        (_, true) => continue 'line,
                        _ => break 'line,
                    }
                }
            }

            let name_len = match &attr.name {
                AttributeName::BuiltIn(name) => {
                    let name = name.to_string();
                    name.len()
                }
                AttributeName::Custom(name) => name.value().len() + 2,
                AttributeName::Spread(_) => unreachable!(),
            };
            total += name_len;

            //
            if attr.can_be_shorthand() {
                total += 2;
            } else {
                total += self.attr_value_len(&attr.value);
            }

            total += 6;
        }

        for spread in spreads {
            let expr_len = self.retrieve_formatted_expr(&spread.expr).len();
            total += expr_len + 3;
        }

        total
    }

    #[allow(clippy::map_entry)]
    pub fn retrieve_formatted_expr(&mut self, expr: &Expr) -> &str {
        let loc = expr.span().start();

        if !self.cached_formats.contains_key(&loc) {
            let formatted = self.unparse_expr(expr);
            self.cached_formats.insert(loc, formatted);
        }

        self.cached_formats.get(&loc).unwrap().as_str()
    }

    fn write_for_loop(&mut self, forloop: &ForLoop) -> std::fmt::Result {
        write!(
            self.out,
            "for {} in ",
            forloop.pat.clone().into_token_stream(),
        )?;

        self.write_inline_expr(&forloop.expr)?;

        if forloop.body.is_empty() {
            write!(self.out, "}}")?;
            return Ok(());
        }

        self.write_body_indented(&forloop.body.roots)?;

        self.out.tabbed_line()?;
        write!(self.out, "}}")?;

        Ok(())
    }

    fn write_if_chain(&mut self, ifchain: &IfChain) -> std::fmt::Result {
        // Recurse in place by setting the next chain
        let mut branch = Some(ifchain);

        while let Some(chain) = branch {
            let IfChain {
                if_token,
                cond,
                then_branch,
                else_if_branch,
                else_branch,
                ..
            } = chain;

            write!(self.out, "{} ", if_token.to_token_stream(),)?;

            self.write_inline_expr(cond)?;

            self.write_body_indented(&then_branch.roots)?;

            if let Some(else_if_branch) = else_if_branch {
                // write the closing bracket and else
                self.out.tabbed_line()?;
                write!(self.out, "}} else ")?;

                branch = Some(else_if_branch);
            } else if let Some(else_branch) = else_branch {
                self.out.tabbed_line()?;
                write!(self.out, "}} else {{")?;

                self.write_body_indented(&else_branch.roots)?;
                branch = None;
            } else {
                branch = None;
            }
        }

        self.out.tabbed_line()?;
        write!(self.out, "}}")?;

        Ok(())
    }

    /// An expression within a for or if block that might need to be spread out across several lines
    fn write_inline_expr(&mut self, expr: &Expr) -> std::fmt::Result {
        let unparsed = self.unparse_expr(expr);
        let mut lines = unparsed.lines();
        let first_line = lines.next().unwrap();
        write!(self.out, "{first_line}")?;

        let mut was_multiline = false;

        for line in lines {
            was_multiline = true;
            self.out.tabbed_line()?;
            write!(self.out, "{line}")?;
        }

        if was_multiline {
            self.out.tabbed_line()?;
            write!(self.out, "{{")?;
        } else {
            write!(self.out, " {{")?;
        }

        Ok(())
    }
}

pub(crate) trait SpanLength {
    fn line_length(&self) -> usize;
}
impl SpanLength for Span {
    fn line_length(&self) -> usize {
        self.end().line - self.start().line
    }
}
