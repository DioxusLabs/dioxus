use crate::interperter::build;
use dioxus_core::LazyNodes;
use dioxus_rsx::CallBody;
use syn::{parse_str, Result};

mod attributes;
pub mod captuered_context;
mod elements;
mod interperter;

pub fn rsx_to_html(text: &str, context: &captuered_context::CapturedContext) ->  {
    panic!()
}
