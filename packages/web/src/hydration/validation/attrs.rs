use dioxus_core::{Attribute, AttributeValue, TemplateAttribute};

use super::serialize::is_internal_attribute_name;

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

#[derive(Clone)]
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
    let mut expected = std::collections::BTreeMap::<String, ExpectedAttributeValue>::new();

    for attr in static_attrs {
        if let TemplateAttribute::Static {
            name,
            value,
            namespace,
        } = attr
        {
            if should_skip_attribute(name, *namespace) {
                continue;
            }
            expected.insert(
                (*name).to_string(),
                expected_static_attribute_value(name, value),
            );
        }
    }

    for attrs in dynamic_attrs {
        for attr in attrs.iter() {
            if should_skip_attribute(attr.name, attr.namespace) {
                continue;
            }
            expected.insert(
                attr.name.to_string(),
                expected_dynamic_attribute_value(attr.name, &attr.value),
            );
        }
    }

    let mut actual = std::collections::BTreeMap::<String, String>::new();
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
    namespace.is_some() || is_internal_attribute_name(name)
}

fn str_truthy(value: &str) -> bool {
    !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
}

pub(super) fn is_boolean_html_attribute(name: &str) -> bool {
    BOOLEAN_HTML_ATTRIBUTES.contains(&name)
}
