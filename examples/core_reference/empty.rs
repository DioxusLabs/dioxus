//! Example: Null/None Children
//! ---------------------------
//!
//! This is a simple pattern that allows you to return no elements!

use dioxus::prelude::*;

pub static Example: FC<()> = |(cx, props)| cx.render(rsx! { Fragment {} });
