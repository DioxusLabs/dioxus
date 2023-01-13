use dioxus_rsx::CallBody;

use crate::util::*;
use crate::writer::*;

mod buffer;
mod component;
mod element;
mod expr;
mod util;
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
    let mut last_bracket_end = 0;

    use triple_accel::{levenshtein_search, Match};

    for Match { end, start, k } in levenshtein_search(b"rsx! {", contents.as_bytes()) {
        let open = end;

        if k > 1 {
            continue;
        }

        // ensure the marker is not nested
        if start < last_bracket_end {
            continue;
        }

        let indent_level = {
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

        let remaining = &contents[open - 1..];
        let close = find_bracket_end(remaining).unwrap();
        // Move the last bracket end to the end of this block to avoid nested blocks
        last_bracket_end = close + open - 1;

        // Format the substring, doesn't include the outer brackets
        let substring = &remaining[1..close - 1];

        // make sure to add back whatever weird whitespace there was at the end
        let mut remaining_whitespace = substring.chars().rev().take_while(|c| *c == ' ').count();

        let mut new = fmt_block(substring, indent_level).unwrap();

        // if the new string is not multiline, don't try to adjust the marker ending
        // We want to trim off any indentation that there might be
        if new.len() <= 80 && !new.contains('\n') {
            new = format!(" {new} ");
            remaining_whitespace = 0;
        }

        if new == substring {
            continue;
        }

        formatted_blocks.push(FormattedBlock {
            formatted: new,
            start: open,
            end: last_bracket_end - remaining_whitespace - 1,
        });
    }

    formatted_blocks
}

pub fn write_block_out(body: CallBody) -> Option<String> {
    let mut buf = Writer {
        src: vec!["".to_string()],
        ..Writer::default()
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

    let mut buf = Writer {
        src: block.lines().map(|f| f.to_string()).collect(),
        ..Writer::default()
    };

    buf.out.indent = indent_level;

    // Oneliner optimization
    if buf.is_short_children(&body.roots).is_some() {
        buf.write_ident(&body.roots[0]).unwrap();
    } else {
        buf.write_body_indented(&body.roots).unwrap();
    }

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
