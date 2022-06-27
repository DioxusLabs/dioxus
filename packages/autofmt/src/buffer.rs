use std::fmt::{Result, Write};

use dioxus_rsx::BodyNode;

pub struct Buffer {
    pub buf: String,
    pub line: usize,
    pub indent: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            line: 0,
            indent: 0,
        }
    }

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

    pub fn write_indented_ident(&mut self, lines: &[&str], node: &BodyNode) -> Result {
        self.write_ident(lines, node)?;
        Ok(())
    }

    pub fn write_ident(&mut self, lines: &[&str], node: &BodyNode) -> Result {
        match node {
            BodyNode::Element(el) => self.write_element(el, lines),
            BodyNode::Component(component) => self.write_component(component, lines),
            BodyNode::Text(text) => self.write_text(text),
            BodyNode::RawExpr(exp) => self.write_raw_expr(exp, lines),
        }
    }

    pub fn write_text(&mut self, text: &syn::LitStr) -> Result {
        write!(self.buf, "\"{}\"", text.value())
    }

    // Push out the indent level and write each component, line by line
    pub fn write_body_indented(&mut self, children: &[BodyNode], lines: &[&str]) -> Result {
        self.indent += 1;
        for child in children {
            // Exprs handle their own indenting/line breaks
            if !matches!(child, BodyNode::RawExpr(_)) {
                self.tabbed_line()?;
            }

            self.write_ident(lines, child)?;
        }
        self.indent -= 1;
        Ok(())
    }
}
