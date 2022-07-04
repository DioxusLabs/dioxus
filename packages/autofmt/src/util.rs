use dioxus_rsx::*;

// todo: use recursive or complete sizeing
pub fn extract_attr_len(attributes: &[ElementAttrNamed]) -> usize {
    attributes
        .iter()
        .map(|attr| match &attr.attr {
            ElementAttr::AttrText { value, .. } => value.value().len(),
            ElementAttr::AttrExpression { .. } => 10,
            ElementAttr::CustomAttrText { value, .. } => value.value().len(),
            ElementAttr::CustomAttrExpression { .. } => 10,
            ElementAttr::EventTokens { .. } => 1000000,
        })
        .sum()
}

pub fn find_bracket_end(contents: &str) -> Option<usize> {
    let mut depth = 0;

    for (i, c) in contents.chars().enumerate() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;

            if depth == 0 {
                return Some(i);
            }
        }
    }

    None
}
