use dioxus_core::AttributeValue;
use std::collections::{BTreeMap, BTreeSet};

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

pub(crate) type SnapshotAttrs = BTreeMap<(String, Option<String>), String>;
pub(crate) type SnapshotListeners = BTreeSet<String>;

pub(crate) fn set_attr(
    attrs: &mut SnapshotAttrs,
    name: String,
    namespace: Option<String>,
    value: String,
) {
    attrs.insert((name, namespace), value);
}

pub(crate) fn remove_attr(attrs: &mut SnapshotAttrs, name: &str, namespace: Option<&str>) {
    attrs.remove(&(name.to_string(), namespace.map(ToString::to_string)));
}

pub(crate) fn snapshot_attrs(attrs: &SnapshotAttrs) -> Vec<SnapshotAttr> {
    attrs
        .iter()
        .map(|((name, namespace), value)| SnapshotAttr {
            name: name.clone(),
            namespace: namespace.clone(),
            value: value.clone(),
        })
        .collect()
}

pub(crate) fn snapshot_listeners(listeners: &SnapshotListeners) -> Vec<String> {
    listeners.iter().cloned().collect()
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
