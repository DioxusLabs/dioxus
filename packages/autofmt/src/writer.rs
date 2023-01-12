use dioxus_rsx::{BodyNode, ElementAttr, ElementAttrNamed};
use proc_macro2::{LineColumn, Span};
use std::{
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};
use syn::{spanned::Spanned, Expr};

use crate::buffer::Buffer;

#[derive(Debug, Default)]
pub struct Writer {
    pub src: Vec<String>,
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

impl Writer {
    // Expects to be written directly into place
    pub fn write_ident(&mut self, node: &BodyNode) -> Result {
        match node {
            BodyNode::Element(el) => self.write_element(el),
            BodyNode::Component(component) => self.write_component(component),
            BodyNode::Text(text) => self.out.write_text(text),
            BodyNode::RawExpr(exp) => self.write_raw_expr(exp),
            _ => Ok(()),
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
                    value.source.as_ref().unwrap().value().len() + name.span().line_length() + 3
                }
                ElementAttr::AttrExpression { name, value } => {
                    value.span().line_length() + name.span().line_length() + 3
                }
                ElementAttr::CustomAttrText { value, name } => {
                    value.source.as_ref().unwrap().value().len() + name.value().len() + 3
                }
                ElementAttr::CustomAttrExpression { name, value } => {
                    name.value().len() + value.span().line_length() + 3
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

                    len + name.span().line_length() + 3
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
}

trait SpanLength {
    fn line_length(&self) -> usize;
}
impl SpanLength for Span {
    fn line_length(&self) -> usize {
        self.end().line - self.start().line
    }
}
