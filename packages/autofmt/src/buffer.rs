use std::{
    collections::HashMap,
    fmt::{Result, Write},
};

use dioxus_rsx::{BodyNode, ElementAttr, ElementAttrNamed};
use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;

#[derive(Default, Debug)]
pub struct Buffer {
    pub src: Vec<String>,
    pub cached_formats: HashMap<Location, String>,
    pub buf: String,
    pub indent: usize,
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

    pub fn write_text(&mut self, text: &syn::LitStr) -> Result {
        write!(self.buf, "\"{}\"", text.value())
    }

    pub fn consume(self) -> Option<String> {
        Some(self.buf)
    }

    // Push out the indent level and write each component, line by line
    pub fn write_body_indented(&mut self, children: &[BodyNode]) -> Result {
        self.indent += 1;

        let mut comments = Vec::new();

        for child in children {
            // Exprs handle their own indenting/line breaks
            if !matches!(child, BodyNode::RawExpr(_)) {
                // collect all comments upwards
                let start = child.span().start();
                let line_start = start.line;

                // make sure the comments are actually relevant to this element.
                let this_line = self.src[line_start - 1].as_str();

                let beginning = if this_line.len() > start.column {
                    this_line[..start.column].trim()
                } else {
                    ""
                };

                if beginning.is_empty() {
                    for (id, line) in self.src[..line_start - 1].iter().enumerate().rev() {
                        if line.trim().starts_with("//") || line.is_empty() {
                            comments.push(id);
                        } else {
                            break;
                        }
                    }
                }

                if comments.len() == 1 && self.src[comments[0]].is_empty() {
                    comments.pop();
                }

                let mut last_was_empty = false;
                for comment_line in comments.drain(..).rev() {
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

                self.tabbed_line()?;
            }

            self.write_ident(child)?;
        }

        self.indent -= 1;
        Ok(())
    }

    pub(crate) fn is_short_attrs(&mut self, attributes: &[ElementAttrNamed]) -> usize {
        attributes
            .iter()
            .map(|attr| match &attr.attr {
                ElementAttr::AttrText { value, name } => {
                    value.value().len() + name.span().line_length() + 3
                }
                ElementAttr::AttrExpression { name, value } => {
                    value.span().line_length() + name.span().line_length() + 3
                }
                ElementAttr::CustomAttrText { value, name } => {
                    value.value().len() + name.value().len() + 3
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
            })
            .sum()
    }

    pub fn retrieve_formatted_expr(&mut self, location: LineColumn) -> Option<String> {
        self.cached_formats.remove(&Location::new(location))
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
