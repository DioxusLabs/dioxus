use crate::{util::*, write_ident};
use dioxus_rsx::*;
use std::fmt::Write;
use triple_accel::{levenshtein_search, Match};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq, Hash)]
pub struct FormattedBlock {
    pub formatted: String,
    pub start: usize,
    pub end: usize,
}

pub fn get_format_blocks(contents: &str) -> Vec<FormattedBlock> {
    let matches = levenshtein_search(b"rsx! {", contents.as_bytes()).peekable();

    let mut formatted_blocks = Vec::new();
    let mut last_bracket_end = 0;

    // find the rsx! marker
    for Match { start, end, k } in matches {
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

        // if we have code to push, we want the code to end up on the right lines with the right indentation
        let mut output = String::new();
        writeln!(output).unwrap();

        for line in new.lines() {
            writeln!(output, "{}", line).ok();
        }

        formatted_blocks.push(FormattedBlock {
            formatted: output,
            start: end,
            end: end + bracket_end - 1,
        });
    }

    formatted_blocks
}

struct Isolate<'a> {
    contents: &'a str,
    start: usize,
    end: usize,
}
fn isolate_body_of_rsx(contents: &str, Match { start, end, k }: triple_accel::Match) -> Isolate {
    todo!()
}

pub fn fmt_block(block: &str) -> Option<String> {
    let mut buf = String::new();
    let lines = block.split('\n').collect::<Vec<_>>();

    for node in &syn::parse_str::<CallBody>(block).ok()?.roots {
        write_ident(&mut buf, &lines, node, 0).ok()?;
    }

    Some(buf)
}

#[test]
fn format_block_basic() {
    let block = r#"
        let a = 120;

        rsx! {
            div {
                h1 { "thing" }
                h1 { "thing" }
                h1 { "thing" }
                h1 { "thing" "is whack" "but you're wacker" }
                h1 { "thing" div {"special cases?"     } }
                div {
                    a: 123,
                    b: 456,
                    c: 789,
                    d: "hello",
                }
                div {
                    a: 123,
                    b: 456,
                    c: 789,
                    d: "hello",
                    p {}
                    c {}
                }

                h3 { class: "asdasd", "asdasd" }
                h3 { class: "mx-large bg-gray-900 tall-md-400", div {
                    "classy"
                }}


                // Some comments explaining my genius
                div {
                    "comment compression"
                }

                // Some comments explaining my genius
                div {
                    // Some comments explaining my genius
                    a: 123,

                    // comment compression
                    b: 456,

                    // comments on attributes
                    c: 789,
                }
            }
        }
    "#;

    // div { class: "asdasd", p { "hello!" } }

    let edits = get_format_blocks(block);

    println!("{}", edits[0].formatted);
}
