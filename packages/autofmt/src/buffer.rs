use std::fmt::{Result, Write};

use dioxus_rsx::BodyNode;

#[derive(Default, Debug)]
pub struct Buffer {
    pub src: Vec<String>,
    pub buf: String,
    pub indent: usize,
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

    pub fn write_indented_ident(&mut self, node: &BodyNode) -> Result {
        self.write_ident(node)?;
        Ok(())
    }

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
                let start = child.span().start().line;

                for (id, line) in self.src[..start - 1].iter().enumerate().rev() {
                    if line.trim().starts_with("//") || line.is_empty() {
                        comments.push(id);
                    } else {
                        break;
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
}
