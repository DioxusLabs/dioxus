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

        // let mut children_iter = children.iter().enumerate().peekable();

        // while let Some((id, child)) = children_iter.next() {
        //     let mut written_empty = false;

        //     for line in lines[self.line..child.span().start().line - 1].iter() {
        //         if id == 0 && line.trim().is_empty() {
        //             continue;
        //         }

        //         if !written_empty && line.is_empty() {
        //             writeln!(self.buf)?;
        //             written_empty = true;
        //             continue;
        //         }

        //         if written_empty && line.is_empty() {
        //             continue;
        //         }

        //         writeln!(self.buf)?;
        //         // self.write_tabs(indent + 1)?;
        //         write!(self.buf, "{}", line.trim())?;
        //     }

        //     writeln!(self.buf)?;
        //     self.indented_tab()?;
        //     self.write_ident(lines, child)?;

        //     self.line = child.span().end().line;
        // }

        // for child in children {
        //     // Exprs handle their own indenting/line breaks
        //     if !matches!(child, BodyNode::RawExpr(_)) {
        //         self.tabbed_line()?;
        //     }

        //     self.write_ident(lines, child)?;
        // }
        self.indent -= 1;
        Ok(())
    }
}

// ShortOptimization::PropsOnTop => {
//     write!(self.buf, " ")?;
//     self.write_attributes(attributes, true, indent)?;

//     if !children.is_empty() && !attributes.is_empty() {
//         write!(self.buf, ",")?;
//     }

//     if let Some(last_attr) = attributes.last() {
//         self.cur_line = last_attr.name_span().end().line + 1;
//     }

//     // write the children
//     for (id, child) in children.iter().enumerate() {
//         let mut written_empty = false;
//         for line in self.lines[self.cur_line..child.span().start().line - 1].iter() {
//             if id == 0 && line.trim().is_empty() {
//                 continue;
//             }

//             if !written_empty && line.is_empty() {
//                 writeln!(self.buf)?;
//                 written_empty = true;
//                 continue;
//             }

//             if written_empty && line.is_empty() {
//                 continue;
//             }

//             writeln!(self.buf)?;
//             // self.write_tabs(indent + 1)?;
//             write!(self.buf, "{}", line.trim())?;
//         }

//         writeln!(self.buf)?;
//         self.write_tabs(indent + 1)?;
//         self.write_ident(child, indent + 1)?;

//         self.cur_line = child.span().end().line;
//     }

//     writeln!(self.buf)?;
//     self.write_tabs(indent)?;
//     write!(self.buf, "}}")?;
// }
