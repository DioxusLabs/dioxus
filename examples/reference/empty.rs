//! Example: Null/None Children
//! ---------------------------
//!
//! This is a simple pattern that allows you to return no elements!

fn main() {}
use dioxus::prelude::*;
static Example: FC<()> = |cx| cx.render(rsx! { Fragment {} });
