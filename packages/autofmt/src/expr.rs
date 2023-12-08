//! pretty printer for rsx!
use std::fmt::{Result, Write};

use proc_macro2::Span;

use crate::{collect_macros::byte_offset, Writer};

impl Writer<'_> {
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
        write!(self.out, "{}", &first_line[start.column - 1..].trim_start())?;

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
}
