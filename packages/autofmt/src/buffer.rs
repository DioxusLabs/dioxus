//! The output buffer that supports some helpful methods
//! These are separate from the input so we can lend references between the two
//!
//!
//!

use std::fmt::{Result, Write};

use dioxus_rsx::IfmtInput;

use crate::{indent::IndentOptions, write_ifmt};

/// The output buffer that tracks indent and string
#[derive(Debug, Default)]
pub struct Buffer {
    pub buf: String,
    pub indent_level: usize,
    pub indent: IndentOptions,
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
        self.write_tabs(self.indent_level)
    }

    pub fn indented_tab(&mut self) -> Result {
        self.write_tabs(self.indent_level + 1)
    }

    pub fn write_tabs(&mut self, num: usize) -> std::fmt::Result {
        for _ in 0..num {
            write!(self.buf, "{}", self.indent.indent_str())?
        }
        Ok(())
    }

    pub fn new_line(&mut self) -> Result {
        writeln!(self.buf)
    }

    pub fn write_text(&mut self, text: &IfmtInput) -> Result {
        write_ifmt(text, &mut self.buf)
    }
}

impl std::fmt::Write for Buffer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }
}
