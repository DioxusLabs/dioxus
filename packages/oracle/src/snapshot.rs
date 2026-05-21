use dioxus_core::AttributeValue;

/// A stable, comparable view of the mock renderer tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotNode {
    Element {
        tag: String,
        namespace: Option<String>,
        attrs: Vec<SnapshotAttr>,
        listeners: Vec<String>,
        children: Vec<SnapshotNode>,
    },
    Text(String),
}

/// A stable attribute snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotAttr {
    pub name: String,
    pub namespace: Option<String>,
    pub value: String,
}
pub(crate) fn attr_key(attr: &SnapshotAttr) -> (&str, Option<&str>) {
    (attr.name.as_str(), attr.namespace.as_deref())
}

pub(crate) fn attr_to_string(value: &AttributeValue) -> Option<String> {
    match value {
        AttributeValue::Text(s) => Some(s.clone()),
        AttributeValue::Bool(b) => Some(b.to_string()),
        AttributeValue::Float(f) => Some(f.to_string()),
        AttributeValue::Int(i) => Some(i.to_string()),
        AttributeValue::None => None,
        _ => Some("<opaque>".to_string()),
    }
}
