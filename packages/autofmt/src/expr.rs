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
        let num_spaces_desired = (self.indent * 4) as isize;

        let first = &self.src[start.line - 1];
        let num_spaces_real = first.chars().take_while(|c| c.is_whitespace()).count() as isize;

        let offset = num_spaces_real - num_spaces_desired;

        for line in &self.src[start.line - 1..end.line] {
            writeln!(self.buf)?;
            // trim the leading whitespace
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
