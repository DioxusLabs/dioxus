use dioxus_rsx::CallBody;

use crate::buffer::*;
use crate::util::*;

mod buffer;
mod component;
mod element;
mod expr;
mod util;

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
    let mut last_bracket_end = 0;

    use triple_accel::{levenshtein_search, Match};

    for Match { end, start, k } in levenshtein_search(b"rsx! {", contents.as_bytes()) {
        if k > 1 {
            continue;
        }

        // ensure the marker is not nested
        if start < last_bracket_end {
            continue;
        }

        let mut indent_level = {
            // walk backwards from start until we find a new line
            let mut lines = contents[..start].lines().rev();
            match lines.next() {
                Some(line) => {
                    if line.starts_with("//") || line.starts_with("///") {
                        continue;
                    }

                    line.chars().take_while(|c| *c == ' ').count() / 4
                }
                None => 0,
            }
        };

        let remaining = &contents[end - 1..];
        let bracket_end = find_bracket_end(remaining).unwrap();
        let sub_string = &contents[end..bracket_end + end - 1];
        last_bracket_end = bracket_end + end - 1;

        let mut new = fmt_block(sub_string, indent_level).unwrap();

        if new.len() <= 80 && !new.contains('\n') {
            new = format!(" {new} ");

            // if the new string is not multiline, don't try to adjust the marker ending
            // We want to trim off any indentation that there might be
            indent_level = 0;
        }

        let end_marker = end + bracket_end - indent_level * 4 - 1;

        if new == contents[end..end_marker] {
            continue;
        }

        formatted_blocks.push(FormattedBlock {
            formatted: new,
            start: end,
            end: end_marker,
        });
    }

    formatted_blocks
}

pub fn write_block_out(body: CallBody) -> Option<String> {
    let mut buf = Buffer {
        src: vec![],
        indent: 0,
        ..Buffer::default()
    };

    // Oneliner optimization
    if buf.is_short_children(&body.roots).is_some() {
        buf.write_ident(&body.roots[0]).unwrap();
    } else {
        buf.write_body_indented(&body.roots).unwrap();
    }

    buf.consume()
}

pub fn fmt_block(block: &str, indent_level: usize) -> Option<String> {
    let body = syn::parse_str::<dioxus_rsx::CallBody>(block).unwrap();

    let mut buf = Buffer {
        src: block.lines().map(|f| f.to_string()).collect(),
        indent: indent_level,
        ..Buffer::default()
    };

    // Oneliner optimization
    if buf.is_short_children(&body.roots).is_some() {
        buf.write_ident(&body.roots[0]).unwrap();
    } else {
        buf.write_body_indented(&body.roots).unwrap();
    }

    // writing idents leaves the final line ended at the end of the last ident
    if buf.buf.contains('\n') {
        buf.new_line().unwrap();
    }

    buf.consume()
}

pub fn apply_format(input: &str, block: FormattedBlock) -> String {
    let start = block.start;
    let end = block.end;

    let (left, _) = input.split_at(start);
    let (_, right) = input.split_at(end);

    // dbg!(&block.formatted);

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
