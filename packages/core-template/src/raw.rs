/// A compact static tree of template structure.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateRawTree {
    /// No template structure.
    Empty,
    /// A sequence of template trees.
    Sequence(&'static [&'static TemplateRawTree]),
    /// An element with static attributes and children.
    Element {
        /// Static tag name.
        tag: &'static str,
        /// Optional element namespace.
        namespace: Option<&'static str>,
        /// Static and dynamic attributes.
        attrs: &'static TemplateRawTree,
        /// Child nodes.
        children: &'static TemplateRawTree,
    },
    /// A static attribute.
    StaticAttr {
        /// Static attribute name.
        name: &'static str,
        /// Static attribute value.
        value: &'static str,
        /// Optional attribute namespace.
        namespace: Option<&'static str>,
    },
    /// A dynamic attribute slot.
    DynamicAttr,
    /// A static text node.
    StaticText(&'static str),
    /// A dynamic node slot.
    DynamicNode,
}
