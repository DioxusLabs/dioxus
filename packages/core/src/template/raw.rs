/// Static attribute namespace information in a raw template tape.
#[doc(hidden)]
pub type TemplateRawAttrNamespace = Option<&'static str>;

/// One unlowered operation in a template tape.
///
/// The RSX macro emits this raw tape directly. [`TemplateStorage::build`] lowers it into packed
/// [`TemplateOp`]s and dynamic [`TemplatePath`]s in const context.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateRawOp {
    /// Open an element.
    OpenElement {
        /// Static tag name.
        tag: &'static str,
        /// Optional element namespace.
        namespace: Option<&'static str>,
    },
    /// Close the current element.
    CloseElement,
    /// Static attribute on the current element.
    StaticAttr {
        /// Static attribute name.
        name: &'static str,
        /// Static attribute value.
        value: &'static str,
        /// Attribute namespace.
        namespace: TemplateRawAttrNamespace,
    },
    /// Dynamic attribute slot on the current element.
    DynamicAttr,
    /// Static text node.
    StaticText {
        /// Static text value.
        value: &'static str,
    },
    /// Dynamic node slot.
    DynamicNode,
}

impl TemplateRawOp {
    /// Create an open-element raw op.
    pub const fn open_element(tag: &'static str, namespace: Option<&'static str>) -> Self {
        Self::OpenElement { tag, namespace }
    }

    /// Create a close-element raw op.
    pub const fn close_element() -> Self {
        Self::CloseElement
    }

    /// Create a dynamic-attribute raw op.
    pub const fn dynamic_attr() -> Self {
        Self::DynamicAttr
    }

    /// Create a static-text raw op.
    pub const fn static_text(value: &'static str) -> Self {
        Self::StaticText { value }
    }

    /// Create a dynamic-node raw op.
    pub const fn dynamic_node() -> Self {
        Self::DynamicNode
    }
}
