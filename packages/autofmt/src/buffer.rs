use std::{
    collections::{HashMap, VecDeque},
    fmt::{Result, Write},
};

use dioxus_rsx::{BodyNode, ElementAttr, ElementAttrNamed, IfmtInput};
use proc_macro2::{LineColumn, Span};
use syn::{spanned::Spanned, Expr};

#[derive(Default, Debug)]
pub struct Buffer {
    pub src: Vec<String>,
    pub cached_formats: HashMap<Location, String>,
    pub buf: String,
    pub indent: usize,
    pub comments: VecDeque<usize>,
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

impl Buffer {
    // Create a new line and tab it to the current tab level
    pub fn tabbed_line(&mut self) -> Result {
        self.new_line()?;
        self.tab()
    }

    // Create a new line and tab it to the current tab level
    pub fn indented_tabbed_line(&mut self) -> Result {
        self.new_line()?;
        self.indented_tab()
    }

    pub fn tab(&mut self) -> Result {
        self.write_tabs(self.indent)
    }

    pub fn indented_tab(&mut self) -> Result {
        self.write_tabs(self.indent + 1)
    }

    pub fn write_tabs(&mut self, num: usize) -> std::fmt::Result {
        for _ in 0..num {
            write!(self.buf, "    ")?
        }
        Ok(())
    }

    pub fn new_line(&mut self) -> Result {
        writeln!(self.buf)
    }

    // Expects to be written directly into place
    pub fn write_ident(&mut self, node: &BodyNode) -> Result {
        match node {
            BodyNode::Element(el) => self.write_element(el),
            BodyNode::Component(component) => self.write_component(component),
            BodyNode::Text(text) => self.write_text(text),
            BodyNode::RawExpr(exp) => self.write_raw_expr(exp),
        }
    }

    pub fn write_text(&mut self, text: &IfmtInput) -> Result {
        write!(self.buf, "\"{}\"", text.source.as_ref().unwrap().value())
    }

    pub fn consume(self) -> Option<String> {
        Some(self.buf)
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
                    self.new_line()?;
                }
                last_was_empty = true;
            } else {
                last_was_empty = false;
                self.tabbed_line()?;
                write!(self.buf, "{}", self.src[comment_line].trim())?;
            }
        }

        Ok(())
    }

    // Push out the indent level and write each component, line by line
    pub fn write_body_indented(&mut self, children: &[BodyNode]) -> Result {
        self.indent += 1;

        self.write_body_no_indent(children)?;

        self.indent -= 1;
        Ok(())
    }

    pub fn write_body_no_indent(&mut self, children: &[BodyNode]) -> Result {
        for child in children {
            // Exprs handle their own indenting/line breaks
            if !matches!(child, BodyNode::RawExpr(_)) {
                if self.current_span_is_primary(child.span()) {
                    self.write_comments(child.span())?;
                }
                self.tabbed_line()?;
            }

            self.write_ident(child)?;
        }

        Ok(())
    }

    pub(crate) fn is_short_attrs(&mut self, attributes: &[ElementAttrNamed]) -> usize {
        let mut total = 0;

        for attr in attributes {
            if self.current_span_is_primary(attr.attr.flart()) {
                'line: for line in self.src[..attr.attr.flart().start().line - 1].iter().rev() {
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
