use crate::{util::*, FormattedBlock};
use dioxus_rsx::*;
use std::fmt::Write;
use triple_accel::{levenshtein_search, Match};

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

    // let edits = get_format_blocks(block);

    // println!("{}", edits[0].formatted);
}
