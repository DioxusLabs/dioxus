#![allow(dead_code)]

use proc_macro2::Span;

#[derive(Debug, Default)]
pub struct Whitespace {
    span_start: Option<Span>,
    span_end: Option<Span>,
}

// right now we dont care if whitespace is different, sorry
impl Eq for Whitespace {}
impl PartialEq for Whitespace {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
