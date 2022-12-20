//! pretty printer for rsx!
use std::fmt::{Result, Write};

use crate::Buffer;

impl Buffer {
    pub fn write_raw_expr(&mut self, exp: &syn::Expr) -> Result {
        /*
        We want to normalize the expr to the appropriate indent level.
        */

        // in a perfect world, just fire up the rust pretty printer
        // pretty_print_rust_code_as_if_it_were_rustfmt()

        use syn::spanned::Spanned;
        let placement = exp.span();
        let start = placement.start();
        let end = placement.end();
        // let num_spaces_desired = (self.indent * 4) as isize;

        // print comments
        // let mut queued_comments = vec![];
        // let mut offset = 2;
        // loop {
        //     let line = &self.src[start.line - offset];
        //     if line.trim_start().starts_with("//") {
        //         queued_comments.push(line);
        //     } else {
        //         break;
        //     }

        //     offset += 1;
        // }
        // let had_comments = !queued_comments.is_empty();
        // for comment in queued_comments.into_iter().rev() {
        //     writeln!(self.buf, "{}", comment)?;
        // }

        // if the expr is on one line, just write it directly
        if start.line == end.line {
            write!(
                self.buf,
                "{}",
                &self.src[start.line - 1][start.column - 1..end.column].trim()
            )?;
            return Ok(());
        }

        // If the expr is multiline, we want to collect all of its lines together and write them out properly
        // This involves unshifting the first line if it's aligned
        let first_line = &self.src[start.line - 1];
        write!(
            self.buf,
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
            writeln!(self.buf)?;
            // trim the leading whitespace
            let line = match id {
                x if x == (end.line - start.line) - 1 => &line[..end.column],
                _ => line,
            };

            if offset < 0 {
                for _ in 0..-offset {
                    write!(self.buf, " ")?;
                }

                write!(self.buf, "{}", line)?;
            } else {
                let offset = offset as usize;
                let right = &line[offset..];
                write!(self.buf, "{}", right)?;
            }
        }

        // let first = &self.src[start.line - 1];
        // let num_spaces_real = first.chars().take_while(|c| c.is_whitespace()).count() as isize;
        // let offset = num_spaces_real - num_spaces_desired;

        // for (row, line) in self.src[start.line - 1..end.line].iter().enumerate() {
        //     let line = match row {
        //         0 => &line[start.column - 1..],
        //         a if a == (end.line - start.line) => &line[..end.column - 1],
        //         _ => line,
        //     };

        //     writeln!(self.buf)?;
        //     // trim the leading whitespace
        //     if offset < 0 {
        //         for _ in 0..-offset {
        //             write!(self.buf, " ")?;
        //         }

        //         write!(self.buf, "{}", line)?;
        //     } else {
        //         let offset = offset as usize;
        //         let right = &line[offset..];
        //         write!(self.buf, "{}", right)?;
        //     }
        // }

        Ok(())
    }
}

// :(
// fn pretty_print_rust_code_as_if_it_were_rustfmt(code: &str) -> String {
//     let formatted = prettyplease::unparse_expr(exp);
//     for line in formatted.lines() {
//         write!(self.buf, "{}", line)?;
//         self.new_line()?;
//     }
// }
