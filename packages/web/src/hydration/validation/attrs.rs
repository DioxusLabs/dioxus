use std::{collections::BTreeMap, fmt::Write};

use dioxus_core::{Attribute, AttributeValue, TemplateAttribute};

use super::serialize::is_internal_attribute_name;

const DANGEROUS_INNER_HTML_ATTRIBUTE: &str = "dangerous_inner_html";
const STYLE_NAMESPACE: &str = "style";

const BOOLEAN_HTML_ATTRIBUTES: &[&str] = &[
    "allowfullscreen",
    "allowpaymentrequest",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "controls",
    "default",
    "defer",
    "disabled",
    "formnovalidate",
    "hidden",
    "ismap",
    "itemscope",
    "loop",
    "multiple",
    "muted",
    "nomodule",
    "novalidate",
    "open",
    "playsinline",
    "readonly",
    "required",
    "reversed",
    "selected",
    "truespeed",
    "webkitdirectory",
];

#[derive(Default)]
pub(super) struct AttributeMismatches {
    missing: Vec<String>,
    unexpected: Vec<String>,
    mismatched: Vec<String>,
}

impl AttributeMismatches {
    pub(super) fn has_mismatches(&self) -> bool {
        !self.missing.is_empty() || !self.unexpected.is_empty() || !self.mismatched.is_empty()
    }

    pub(super) fn describe(&self) -> String {
        let mut parts = Vec::new();

        if !self.missing.is_empty() {
            parts.push(format!("the DOM is missing [{}]", self.missing.join(", ")));
        }

        if !self.unexpected.is_empty() {
            parts.push(format!(
                "the DOM has unexpected [{}]",
                self.unexpected.join(", ")
            ));
        }

        if !self.mismatched.is_empty() {
            parts.push(format!(
                "these values differ [{}]",
                self.mismatched.join(", ")
            ));
        }

        parts.join("; ")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ExpectedAttributeValue {
    Absent,
    Present,
    Exact(String),
    PresenceOnly,
}

pub(super) fn find_attribute_mismatches(
    element: &web_sys::Element,
    static_attrs: &'static [TemplateAttribute],
    dynamic_attrs: &[&[Attribute]],
) -> AttributeMismatches {
    let mut mismatches = AttributeMismatches::default();
    let expected = expected_attributes(static_attrs, dynamic_attrs);

    let mut actual = BTreeMap::<String, String>::new();
    let names = element.get_attribute_names();
    for idx in 0..names.length() {
        let Some(name) = names.get(idx).as_string() else {
            continue;
        };
        if should_skip_attribute(&name, None) {
            continue;
        }
        actual.insert(
            name.clone(),
            element.get_attribute(&name).unwrap_or_default(),
        );
    }

    for (name, expected_value) in &expected {
        match expected_value {
            ExpectedAttributeValue::Absent => {
                if actual.contains_key(name) {
                    mismatches.unexpected.push(name.clone());
                }
            }
            ExpectedAttributeValue::Present | ExpectedAttributeValue::PresenceOnly => {
                if !actual.contains_key(name) {
                    mismatches.missing.push(name.clone());
                }
            }
            ExpectedAttributeValue::Exact(expected_value) => match actual.get(name) {
                None => mismatches.missing.push(name.clone()),
                Some(actual_value) if actual_value != expected_value => {
                    mismatches.mismatched.push(format!(
                        "{}: expected {:?}, found {:?}",
                        name, expected_value, actual_value
                    ))
                }
                Some(_) => {}
            },
        }
    }

    for name in actual.keys() {
        if !expected.contains_key(name) {
            mismatches.unexpected.push(name.clone());
        }
    }

    mismatches.missing.sort();
    mismatches.missing.dedup();
    mismatches.unexpected.sort();
    mismatches.unexpected.dedup();
    mismatches
}

fn expected_attributes(
    static_attrs: &'static [TemplateAttribute],
    dynamic_attrs: &[&[Attribute]],
) -> BTreeMap<String, ExpectedAttributeValue> {
    let mut expected = BTreeMap::new();
    let mut static_styles = String::new();
    let mut dynamic_styles = String::new();

    for attr in static_attrs {
        let TemplateAttribute::Static {
            name,
            value,
            namespace,
        } = attr
        else {
            continue;
        };

        if should_skip_attribute(name, *namespace) {
            continue;
        }

        if *namespace == Some(STYLE_NAMESPACE) {
            append_style_declaration(&mut static_styles, name, value);
            continue;
        }

        expected.insert(
            (*name).to_string(),
            expected_static_attribute_value(name, value),
        );
    }

    for attrs in dynamic_attrs {
        for attr in attrs.iter() {
            if should_skip_attribute(attr.name, attr.namespace) {
                continue;
            }

            if attr.namespace == Some(STYLE_NAMESPACE) {
                append_dynamic_style_declaration(&mut dynamic_styles, attr.name, &attr.value);
                continue;
            }

            expected.insert(
                attr.name.to_string(),
                expected_dynamic_attribute_value(attr.name, &attr.value),
            );
        }
    }

    if !static_styles.is_empty() || !dynamic_styles.is_empty() {
        expected.insert(
            STYLE_NAMESPACE.to_string(),
            ExpectedAttributeValue::Exact(format!("{static_styles}{dynamic_styles}")),
        );
    }

    expected
}

fn expected_static_attribute_value(name: &str, value: &str) -> ExpectedAttributeValue {
    if is_boolean_html_attribute(name) {
        if str_truthy(value) {
            ExpectedAttributeValue::Present
        } else {
            ExpectedAttributeValue::Absent
        }
    } else {
        ExpectedAttributeValue::Exact(value.to_string())
    }
}

fn expected_dynamic_attribute_value(name: &str, value: &AttributeValue) -> ExpectedAttributeValue {
    match value {
        AttributeValue::None => ExpectedAttributeValue::Absent,
        AttributeValue::Text(value) => {
            if is_boolean_html_attribute(name) {
                if str_truthy(value) {
                    ExpectedAttributeValue::Present
                } else {
                    ExpectedAttributeValue::Absent
                }
            } else {
                ExpectedAttributeValue::Exact(value.clone())
            }
        }
        AttributeValue::Float(value) => {
            if is_boolean_html_attribute(name) {
                if *value != 0.0 {
                    ExpectedAttributeValue::Present
                } else {
                    ExpectedAttributeValue::Absent
                }
            } else {
                ExpectedAttributeValue::Exact(value.to_string())
            }
        }
        AttributeValue::Int(value) => {
            if is_boolean_html_attribute(name) {
                if *value != 0 {
                    ExpectedAttributeValue::Present
                } else {
                    ExpectedAttributeValue::Absent
                }
            } else {
                ExpectedAttributeValue::Exact(value.to_string())
            }
        }
        AttributeValue::Bool(value) => {
            if is_boolean_html_attribute(name) {
                if *value {
                    ExpectedAttributeValue::Present
                } else {
                    ExpectedAttributeValue::Absent
                }
            } else {
                ExpectedAttributeValue::Exact(value.to_string())
            }
        }
        AttributeValue::Any(_) => ExpectedAttributeValue::PresenceOnly,
        AttributeValue::Listener(_) => ExpectedAttributeValue::Absent,
    }
}

fn should_skip_attribute(name: &str, namespace: Option<&str>) -> bool {
    is_internal_attribute_name(name)
        || name == DANGEROUS_INNER_HTML_ATTRIBUTE
        || matches!(namespace, Some(ns) if ns != STYLE_NAMESPACE)
}

fn append_style_declaration(into: &mut String, name: &str, value: &str) {
    let _ = write!(into, "{name}:{value};");
}

fn append_dynamic_style_declaration(into: &mut String, name: &str, value: &AttributeValue) {
    let _ = write!(into, "{name}:");
    match value {
        AttributeValue::Text(value) => into.push_str(value),
        AttributeValue::Float(value) => {
            let _ = write!(into, "{value}");
        }
        AttributeValue::Int(value) => {
            let _ = write!(into, "{value}");
        }
        AttributeValue::Bool(value) => {
            let _ = write!(into, "{value}");
        }
        AttributeValue::Any(_) | AttributeValue::Listener(_) | AttributeValue::None => {}
    }
    into.push(';');
}

fn str_truthy(value: &str) -> bool {
    !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
}

pub(super) fn is_boolean_html_attribute(name: &str) -> bool {
    BOOLEAN_HTML_ATTRIBUTES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_inner_html_is_not_treated_as_dom_attribute() {
        const STATIC_ATTRS: &[TemplateAttribute] = &[TemplateAttribute::Static {
            name: DANGEROUS_INNER_HTML_ATTRIBUTE,
            value: "<strong>hello</strong>",
            namespace: None,
        }];

        assert!(expected_attributes(STATIC_ATTRS, &[]).is_empty());
    }

    #[test]
    fn style_namespace_attrs_are_folded_into_style_attribute() {
        const STATIC_ATTRS: &[TemplateAttribute] = &[
            TemplateAttribute::Static {
                name: "width",
                value: "100px",
                namespace: Some(STYLE_NAMESPACE),
            },
            TemplateAttribute::Static {
                name: "height",
                value: "40px",
                namespace: Some(STYLE_NAMESPACE),
            },
        ];
        let dynamic_attrs = vec![
            Attribute::new("display", "block", Some(STYLE_NAMESPACE), false),
            Attribute::new("opacity", 0.5, Some(STYLE_NAMESPACE), false),
        ];
        let dynamic_refs = vec![dynamic_attrs.as_slice()];

        let expected = expected_attributes(STATIC_ATTRS, &dynamic_refs);

        assert_eq!(
            expected.get(STYLE_NAMESPACE),
            Some(&ExpectedAttributeValue::Exact(
                "width:100px;height:40px;display:block;opacity:0.5;".to_string()
            ))
        );
    }
}
