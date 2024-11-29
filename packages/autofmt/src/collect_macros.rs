//! Collect macros from a file
//!
//! Returns all macros that match a pattern. You can use this information to autoformat them later

use proc_macro2::LineColumn;
use syn::{visit::Visit, File, Macro, Meta};

type CollectedMacro<'a> = &'a Macro;

pub fn collect_from_file(file: &File) -> Vec<CollectedMacro<'_>> {
    let mut macros = vec![];
    let mut collector = MacroCollector::new(&mut macros);
    MacroCollector::visit_file(&mut collector, file);
    macros
}

struct MacroCollector<'a, 'b> {
    macros: &'a mut Vec<CollectedMacro<'b>>,
    skip_count: usize,
}

impl<'a, 'b> MacroCollector<'a, 'b> {
    fn new(macros: &'a mut Vec<CollectedMacro<'b>>) -> Self {
        Self {
            macros,
            skip_count: 0,
        }
    }
}

impl<'b> Visit<'b> for MacroCollector<'_, 'b> {
    fn visit_macro(&mut self, i: &'b Macro) {
        // Visit the regular stuff - this will also ensure paths/attributes are visited
        syn::visit::visit_macro(self, i);

        let name = &i.path.segments.last().map(|i| i.ident.to_string());
        if let Some("rsx" | "render") = name.as_deref() {
            if self.skip_count == 0 {
                self.macros.push(i)
            }
        }
    }

    // attributes can occur on stmts and items - we need to make sure the stack is reset when we exit
    // this means we save the skipped length and set it back to its original length
    fn visit_stmt(&mut self, i: &'b syn::Stmt) {
        let skipped_len = self.skip_count;
        syn::visit::visit_stmt(self, i);
        self.skip_count = skipped_len;
    }

    fn visit_item(&mut self, i: &'b syn::Item) {
        let skipped_len = self.skip_count;
        syn::visit::visit_item(self, i);
        self.skip_count = skipped_len;
    }

    fn visit_attribute(&mut self, i: &'b syn::Attribute) {
        // we need to communicate that this stmt is skipped up the tree
        if attr_is_rustfmt_skip(i) {
            self.skip_count += 1;
        }

        syn::visit::visit_attribute(self, i);
    }
}

pub fn byte_offset(input: &str, location: LineColumn) -> usize {
    let mut offset = 0;
    for _ in 1..location.line {
        offset += input[offset..].find('\n').unwrap() + 1;
    }
    offset
        + input[offset..]
            .chars()
            .take(location.column)
            .map(char::len_utf8)
            .sum::<usize>()
}

/// Check if an attribute is a rustfmt skip attribute
fn attr_is_rustfmt_skip(i: &syn::Attribute) -> bool {
    match &i.meta {
        Meta::Path(path) => {
            path.segments.len() == 2
                && matches!(i.style, syn::AttrStyle::Outer)
                && path.segments[0].ident == "rustfmt"
                && path.segments[1].ident == "skip"
        }
        _ => false,
    }
}

#[test]
fn parses_file_and_collects_rsx_macros() {
    let contents = include_str!("../tests/samples/long.rsx");
    let parsed = syn::parse_file(contents).expect("parse file okay");
    let macros = collect_from_file(&parsed);
    assert_eq!(macros.len(), 3);
}

/// Ensure that we only collect non-skipped macros
#[test]
fn dont_collect_skipped_macros() {
    let contents = include_str!("../tests/samples/skip.rsx");
    let parsed = syn::parse_file(contents).expect("parse file okay");
    let macros = collect_from_file(&parsed);
    assert_eq!(macros.len(), 2);
}
