//! Example: Null/None Children
//! ---------------------------
//!
//! This is a simple pattern that allows you to return no elements!

use dioxus::prelude::*;
fn main() {}

static Example: FC<()> = |cx| {
    cx.render(rsx! {
        div {

        }
    })
};
