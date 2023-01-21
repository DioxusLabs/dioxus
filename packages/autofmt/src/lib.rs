use crate::writer::*;
use collect_macros::byte_offset;
use dioxus_rsx::{BodyNode, CallBody};
use proc_macro2::LineColumn;
use syn::{ExprMacro, MacroDelimiter};

mod buffer;
mod collect_macros;
mod component;
mod element;
mod expr;
mod writer;

/// A modification to the original file to be applied by an IDE
///
/// Right now this re-writes entire rsx! blocks at a time, instead of precise line-by-line changes.
///
/// In a "perfect" world we would have tiny edits to preserve things like cursor states and selections. The API here makes
/// it possible to migrate to a more precise modification approach in the future without breaking existing code.
///
/// Note that this is tailored to VSCode's TextEdit API and not a general Diff API. Line numbers are not accurate if
/// multiple edits are applied in a single file without tracking text shifts.
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FormattedBlock {
    /// The new contents of the block
    pub formatted: String,

    /// The line number of the first line of the block.
    pub start: usize,

    /// The end of the block, exclusive.
    pub end: usize,
}

/// Format a file into a list of `FormattedBlock`s to be applied by an IDE for autoformatting.
///
/// This function expects a complete file, not just a block of code. To format individual rsx! blocks, use fmt_block instead.
///
/// The point here is to provide precise modifications of a source file so an accompanying IDE tool can map these changes
/// back to the file precisely.
///
/// Nested blocks of RSX will be handled automatically
pub fn fmt_file(contents: &str) -> Vec<FormattedBlock> {
    let mut formatted_blocks = Vec::new();

    let parsed = syn::parse_file(contents).unwrap();

    let mut macros = vec![];
    collect_macros::collect_from_file(&parsed, &mut macros);

    // No macros, no work to do
    if macros.is_empty() {
        return formatted_blocks;
    }

    let mut writer = Writer::new(contents);

    // Dont parse nested macros
    let mut end_span = LineColumn { column: 0, line: 0 };
    for item in macros {
        let macro_path = &item.path.segments[0].ident;

        // this macro is inside the last macro we parsed, skip it
        if macro_path.span().start() < end_span {
            continue;
        }

        // item.parse_body::<CallBody>();
        let body = item.parse_body::<CallBody>().unwrap();

        let rsx_start = macro_path.span().start();

        writer.out.indent = &writer.src[rsx_start.line - 1]
            .chars()
            .take_while(|c| *c == ' ')
            .count()
            / 4;

        write_body(&mut writer, &body);

        // writing idents leaves the final line ended at the end of the last ident
        if writer.out.buf.contains('\n') {
            writer.out.new_line().unwrap();
            writer.out.tab().unwrap();
        }

        let span = match item.delimiter {
            MacroDelimiter::Paren(b) => b.span,
            MacroDelimiter::Brace(b) => b.span,
            MacroDelimiter::Bracket(b) => b.span,
        };

        let mut formatted = String::new();

        std::mem::swap(&mut formatted, &mut writer.out.buf);

        let start = byte_offset(contents, span.start()) + 1;
        let end = byte_offset(contents, span.end()) - 1;

        // Rustfmt will remove the space between the macro and the opening paren if the macro is a single expression
        let body_is_solo_expr = body.roots.len() == 1
            && matches!(body.roots[0], BodyNode::RawExpr(_) | BodyNode::Text(_));

        if formatted.len() <= 80 && !formatted.contains('\n') && !body_is_solo_expr {
            formatted = format!(" {} ", formatted);
        }

        end_span = span.end();

        if contents[start..end] == formatted {
            continue;
        }

        formatted_blocks.push(FormattedBlock {
            formatted,
            start,
            end,
        });
    }

    formatted_blocks
}

pub fn write_block_out(body: CallBody) -> Option<String> {
    let mut buf = Writer::new("");

    write_body(&mut buf, &body);

    buf.consume()
}

fn write_body(buf: &mut Writer, body: &CallBody) {
    use std::fmt::Write;

    if buf.is_short_children(&body.roots).is_some() {
        // write all the indents with spaces and commas between
        for idx in 0..body.roots.len() - 1 {
            let ident = &body.roots[idx];
            buf.write_ident(ident).unwrap();
            write!(&mut buf.out.buf, ", ").unwrap();
        }

        // write the last ident without a comma
        let ident = &body.roots[body.roots.len() - 1];
        buf.write_ident(ident).unwrap();
    } else {
        buf.write_body_indented(&body.roots).unwrap();
    }
}

pub fn fmt_block_from_expr(raw: &str, expr: ExprMacro) -> Option<String> {
    let body = syn::parse2::<CallBody>(expr.mac.tokens).unwrap();

    let mut buf = Writer::new(raw);

    write_body(&mut buf, &body);

    buf.consume()
}

pub fn fmt_block(block: &str, indent_level: usize) -> Option<String> {
    let body = syn::parse_str::<dioxus_rsx::CallBody>(block).unwrap();

    let mut buf = Writer::new(block);

    buf.out.indent = indent_level;

    write_body(&mut buf, &body);

    // writing idents leaves the final line ended at the end of the last ident
    if buf.out.buf.contains('\n') {
        buf.out.new_line().unwrap();
    }

    buf.consume()
}

pub fn apply_format(input: &str, block: FormattedBlock) -> String {
    let start = block.start;
    let end = block.end;

    let (left, _) = input.split_at(start);
    let (_, right) = input.split_at(end);

    format!("{}{}{}", left, block.formatted, right)
}

// Apply all the blocks
pub fn apply_formats(input: &str, blocks: Vec<FormattedBlock>) -> String {
    let mut out = String::new();

    let mut last = 0;

    for FormattedBlock {
        formatted,
        start,
        end,
    } in blocks
    {
        let prefix = &input[last..start];
        out.push_str(prefix);
        out.push_str(&formatted);
        last = end;
    }

    let suffix = &input[last..];
    out.push_str(suffix);

    out
}
