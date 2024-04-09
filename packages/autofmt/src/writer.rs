use dioxus_rsx::{AttributeType, BodyNode, ElementAttrValue, ForLoop, IfChain, IfmtInput};
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use std::{
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};
use syn::{spanned::Spanned, token::Brace, Expr};

use crate::buffer::Buffer;
use crate::ifmt_to_string;

#[derive(Debug)]
pub struct Writer<'a> {
    pub raw_src: &'a str,
    pub src: Vec<&'a str>,
    pub cached_formats: HashMap<Location, String>,
    pub comments: VecDeque<usize>,
    pub out: Buffer,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}
impl Location {
    pub fn new(start: LineColumn) -> Self {
        Self {
            line: start.line,
            col: start.column,
        }
    }
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

    // Expects to be written directly into place
    pub fn write_ident(&mut self, node: &BodyNode) -> Result {
        match node {
            BodyNode::Element(el) => self.write_element(el),
            BodyNode::Component(component) => self.write_component(component),
            BodyNode::Text(text) => self.out.write_text(text),
            BodyNode::RawExpr(exp) => self.write_raw_expr(exp.span()),
            BodyNode::ForLoop(forloop) => self.write_for_loop(forloop),
            BodyNode::IfChain(ifchain) => self.write_if_chain(ifchain),
        }
    }

    pub fn consume(self) -> Option<String> {
        Some(self.out.buf)
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
        let last_child = children.len();
        let iter = children.iter().peekable().enumerate();

        for (idx, child) in iter {
            if self.current_span_is_primary(child.span()) {
                self.write_comments(child.span())?;
            }

            match child {
                // check if the expr is a short
                BodyNode::RawExpr { .. } => {
                    self.out.tabbed_line()?;
                    self.write_ident(child)?;
                    if idx != last_child - 1 {
                        write!(self.out, ",")?;
                    }
                }
                _ => {
                    self.out.tabbed_line()?;
                    self.write_ident(child)?;
                }
            }
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
            ElementAttrValue::AttrLiteral(lit) => ifmt_to_string(lit).len(),
            ElementAttrValue::Shorthand(expr) => expr.span().line_length(),
            ElementAttrValue::AttrExpr(expr) => {
                let out = self.retrieve_formatted_expr(expr);
                if out.contains('\n') {
                    100000
                } else {
                    out.len()
                }
            }
            ElementAttrValue::EventTokens(tokens) => {
                let as_str = self.retrieve_formatted_expr(tokens);
                if as_str.contains('\n') {
                    100000
                } else {
                    as_str.len()
                }
            }
        }
    }

    pub(crate) fn is_short_attrs(&mut self, attributes: &[AttributeType]) -> usize {
        let mut total = 0;

        // No more than 3 attributes before breaking the line
        if attributes.len() > 3 {
            return 100000;
        }

        for attr in attributes {
            if self.current_span_is_primary(attr.start()) {
                'line: for line in self.src[..attr.start().start().line - 1].iter().rev() {
                    match (line.trim().starts_with("//"), line.is_empty()) {
                        (true, _) => return 100000,
                        (_, true) => continue 'line,
                        _ => break 'line,
                    }
                }
            }

            match attr {
                AttributeType::Named(attr) => {
                    let name_len = match &attr.attr.name {
                        dioxus_rsx::ElementAttrName::BuiltIn(name) => {
                            let name = name.to_string();
                            name.len()
                        }
                        dioxus_rsx::ElementAttrName::Custom(name) => name.value().len() + 2,
                    };
                    total += name_len;

                    //
                    if attr.attr.value.is_shorthand() {
                        total += 2;
                    } else {
                        total += self.attr_value_len(&attr.attr.value);
                    }
                }
                AttributeType::Spread(expr) => {
                    let expr_len = self.retrieve_formatted_expr(expr).len();
                    total += expr_len + 3;
                }
            };

            total += 6;
        }

        total
    }

    #[allow(clippy::map_entry)]
    pub fn retrieve_formatted_expr(&mut self, expr: &Expr) -> &str {
        let loc = Location::new(expr.span().start());

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

        self.write_body_indented(&forloop.body)?;

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

            self.write_body_indented(then_branch)?;

            if let Some(else_if_branch) = else_if_branch {
                // write the closing bracket and else
                self.out.tabbed_line()?;
                write!(self.out, "}} else ")?;

                branch = Some(else_if_branch);
            } else if let Some(else_branch) = else_branch {
                self.out.tabbed_line()?;
                write!(self.out, "}} else {{")?;

                self.write_body_indented(else_branch)?;
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

    pub(crate) fn key_len(&self, key: Option<&IfmtInput>) -> usize {
        match key {
            Some(key) => ifmt_to_string(key).len() + 5,
            None => 0,
        }
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
