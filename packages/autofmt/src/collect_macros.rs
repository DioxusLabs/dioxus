//! Collect macros from a file
//!
//! Returns all macros that match a pattern. You can use this information to autoformat them later

use proc_macro2::LineColumn;
use syn::{visit::Visit, File, Macro};

type CollectedMacro<'a> = &'a Macro;

pub fn collect_from_file(file: &File) -> Vec<CollectedMacro<'_>> {
    let mut macros = vec![];
    MacroCollector::visit_file(
        &mut MacroCollector {
            macros: &mut macros,
        },
        file,
    );
    macros
}

struct MacroCollector<'a, 'b> {
    macros: &'a mut Vec<CollectedMacro<'b>>,
}

impl<'a, 'b> Visit<'b> for MacroCollector<'a, 'b> {
    fn visit_macro(&mut self, i: &'b Macro) {
        if let Some("rsx" | "render") = i
            .path
            .segments
            .last()
            .map(|i| i.ident.to_string())
            .as_deref()
        {
            self.macros.push(i)
        }
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

#[test]
fn parses_file_and_collects_rsx_macros() {
    let contents = include_str!("../tests/samples/long.rsx");
    let parsed = syn::parse_file(contents).unwrap();
    let macros = collect_from_file(&parsed);
    assert_eq!(macros.len(), 3);
}
