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
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Hash)]
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
pub fn fmt_file(contents: &str) -> Vec<FormattedBlock> {
    let mut formatted_blocks = Vec::new();
    let mut last_bracket_end = 0;

    use triple_accel::{levenshtein_search, Match};

    for Match { end, start, .. } in levenshtein_search(b"rsx! {", contents.as_bytes()) {
        // ensure the marker is not nested
        if start < last_bracket_end {
            continue;
        }

        let remaining = &contents[end - 1..];
        let bracket_end = find_bracket_end(remaining).unwrap();
        let sub_string = &contents[end..bracket_end + end - 1];
        last_bracket_end = bracket_end + end - 1;

        let new = fmt_block(sub_string).unwrap();
        let stripped = &contents[end + 1..bracket_end + end - 1];

        if stripped == new {
            continue;
        }

        formatted_blocks.push(FormattedBlock {
            formatted: new,
            start: end,
            end: end + bracket_end - 1,
        });
    }

    formatted_blocks
}

pub fn fmt_block(block: &str) -> Option<String> {
    let mut buf = Buffer {
        src: block.lines().map(|f| f.to_string()).collect(),
        ..Buffer::default()
    };

    let body = syn::parse_str::<dioxus_rsx::CallBody>(block).unwrap();

    buf.write_body_indented(&body.roots).unwrap();

    buf.consume()
}
