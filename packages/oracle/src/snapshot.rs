use dioxus_core::AttributeValue;

/// A stable, comparable view of the mock renderer tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotNode {
    /// An element node in renderer output.
    Element {
        /// The element tag name.
        tag: String,
        /// The element namespace, if any.
        namespace: Option<String>,
        /// The element attributes after renderer-side updates.
        attrs: Vec<SnapshotAttr>,
        /// Event listener names attached to the element.
        listeners: Vec<String>,
        /// Child nodes in document order.
        children: Vec<SnapshotNode>,
    },
    /// A text node.
    Text(String),
}

pub(crate) fn format_snapshot_mismatch(
    message: &str,
    actual: &[SnapshotNode],
    expected: &[SnapshotNode],
) -> String {
    format!("{message}\n\nrenderer snapshot:\n{actual:#?}\n\nexpected snapshot:\n{expected:#?}")
}

/// A stable attribute snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotAttr {
    /// The attribute name.
    pub name: String,
    /// The attribute namespace, if any.
    pub namespace: Option<String>,
    /// The rendered attribute value.
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
