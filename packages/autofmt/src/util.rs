use dioxus_rsx::*;
use std::fmt::Write;

pub fn extract_attr_len(attributes: &[ElementAttrNamed]) -> usize {
    attributes
        .iter()
        .map(|attr| match &attr.attr {
            ElementAttr::AttrText { name, value } => value.value().len(),
            ElementAttr::AttrExpression { name, value } => 10,
            ElementAttr::CustomAttrText { name, value } => value.value().len(),
            ElementAttr::CustomAttrExpression { name, value } => 10,
            ElementAttr::EventTokens { name, tokens } => 1000000,
        })
        .sum()
}

pub fn write_tabs(f: &mut dyn Write, num: usize) -> std::fmt::Result {
    for _ in 0..num {
        write!(f, "    ")?
    }
    Ok(())
}

pub fn find_bracket_end(contents: &str) -> Option<usize> {
    let mut depth = 0;
    let mut i = 0;

    for c in contents.chars() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
        }

        if depth == 0 {
            return Some(i);
        }

        i += 1;
    }

    None
}
