//! pretty printer for rsx!
use std::fmt::{self, Write};

pub fn write_raw_expr(
    exp: &syn::Expr,
    indent: usize,
    lines: &[&str],
    buf: &mut String,
) -> Result<(), fmt::Error> {
    use syn::spanned::Spanned;
    let placement = exp.span();
    let start = placement.start();
    let end = placement.end();
    let num_spaces_desired = (indent * 4) as isize;
    let first = lines[start.line - 1];
    let num_spaces_real = first.chars().take_while(|c| c.is_whitespace()).count() as isize;
    let offset = num_spaces_real - num_spaces_desired;

    for line_id in start.line - 1..end.line {
        let line = lines[line_id];

        // trim the leading whitespace

        if offset < 0 {
            for _ in 0..-offset {
                write!(buf, " ")?;
            }

            writeln!(buf, "{}", line)?;
        } else {
            let offset = offset as usize;

            let right = &line[offset..];
            writeln!(buf, "{}", right)?;
        }
    }

    Ok(())
}
