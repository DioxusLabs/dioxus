use dioxus_rsx::{BodyNode, ElementAttr, ElementAttrNamed, ForLoop};
use proc_macro2::{LineColumn, Span};
use quote::ToTokens;
use std::{
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};
use syn::{spanned::Spanned, Expr, ExprIf};

use crate::buffer::Buffer;

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

    pub fn write_comments(&mut self, child: Span) -> Result {
        // collect all comments upwards
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
        self.out.indent += 1;

        self.write_body_no_indent(children)?;

        self.out.indent -= 1;
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

    pub(crate) fn is_short_attrs(&mut self, attributes: &[ElementAttrNamed]) -> usize {
        let mut total = 0;

        for attr in attributes {
            if self.current_span_is_primary(attr.attr.start()) {
                'line: for line in self.src[..attr.attr.start().start().line - 1].iter().rev() {
                    match (line.trim().starts_with("//"), line.is_empty()) {
                        (true, _) => return 100000,
                        (_, true) => continue 'line,
                        _ => break 'line,
                    }
                }
            }

            total += match &attr.attr {
                ElementAttr::AttrText { value, name } => {
                    value.source.as_ref().unwrap().value().len() + name.span().line_length() + 6
                }
                ElementAttr::AttrExpression { name, value } => {
                    value.span().line_length() + name.span().line_length() + 6
                }
                ElementAttr::CustomAttrText { value, name } => {
                    value.source.as_ref().unwrap().value().len() + name.value().len() + 6
                }
                ElementAttr::CustomAttrExpression { name, value } => {
                    name.value().len() + value.span().line_length() + 6
                }
                ElementAttr::EventTokens { tokens, name } => {
                    let location = Location::new(tokens.span().start());

                    let len = if let std::collections::hash_map::Entry::Vacant(e) =
                        self.cached_formats.entry(location)
                    {
                        let formatted = prettyplease::unparse_expr(tokens);
                        let len = if formatted.contains('\n') {
                            10000
                        } else {
                            formatted.len()
                        };
                        e.insert(formatted);
                        len
                    } else {
                        self.cached_formats[&location].len()
                    };

                    len + name.span().line_length() + 6
                }
            };
        }

        total
    }

    pub fn retrieve_formatted_expr(&mut self, expr: &Expr) -> &str {
        self.cached_formats
            .entry(Location::new(expr.span().start()))
            .or_insert_with(|| prettyplease::unparse_expr(expr))
            .as_str()
    }

    fn write_for_loop(&mut self, forloop: &ForLoop) -> std::fmt::Result {
        write!(
            self.out,
            "for {} in {} {{",
            forloop.pat.clone().into_token_stream(),
            prettyplease::unparse_expr(&forloop.expr)
        )?;

        if forloop.body.is_empty() {
            write!(self.out, "}}")?;
            return Ok(());
        }

        self.write_body_indented(&forloop.body)?;

        self.out.tabbed_line()?;
        write!(self.out, "}}")?;

        Ok(())
    }

    fn write_if_chain(&mut self, ifchain: &ExprIf) -> std::fmt::Result {
        self.write_raw_expr(ifchain.span())
    }
}

trait SpanLength {
    fn line_length(&self) -> usize;
}
impl SpanLength for Span {
    fn line_length(&self) -> usize {
        self.end().line - self.start().line
    }
}
