//! pretty printer for rsx!
use std::fmt::{Result, Write};

use crate::Writer;

impl Writer {
    pub fn write_raw_expr(&mut self, exp: &syn::Expr) -> Result {
        /*
        We want to normalize the expr to the appropriate indent level.
        */

        use syn::spanned::Spanned;
        let placement = exp.span();
        let start = placement.start();
        let end = placement.end();

        // if the expr is on one line, just write it directly
        if start.line == end.line {
            write!(
                self.out,
                "{}",
                &self.src[start.line - 1][start.column - 1..end.column].trim()
            )?;
            return Ok(());
        }

        // If the expr is multiline, we want to collect all of its lines together and write them out properly
        // This involves unshifting the first line if it's aligned
        let first_line = &self.src[start.line - 1];
        write!(
            self.out,
            "{}",
            &first_line[start.column - 1..first_line.len()].trim()
        )?;

        let first_prefix = &self.src[start.line - 1][..start.column];
        let offset = match first_prefix.trim() {
            "" => 0,
            _ => first_prefix
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .count() as isize,
        };

        for (id, line) in self.src[start.line..end.line].iter().enumerate() {
            writeln!(self.out)?;
            // trim the leading whitespace
            let line = match id {
                x if x == (end.line - start.line) - 1 => &line[..end.column],
                _ => line,
            };

            if offset < 0 {
                for _ in 0..-offset {
                    write!(self.out, " ")?;
                }

                write!(self.out, "{}", line)?;
            } else {
                let offset = offset as usize;
                let right = &line[offset..];
                write!(self.out, "{}", right)?;
            }
        }

        Ok(())
    }
}
