//! Core element builder implementation.

use dioxus_core::{
    Attribute, DynamicNode, HasAttributes, IntoAttributeValue, IntoDynNode,
    Template, TemplateAttribute, TemplateNode, VNode,
};
use dioxus_html::events::{MouseData, FormData, FocusData, KeyboardData};
use parking_lot::RwLock;
use std::collections::HashMap;

static TEMPLATES: RwLock<Option<HashMap<(&'static str, Option<&'static str>, usize, bool), Template>>> = RwLock::new(None);

fn get_template(tag: &'static str, namespace: Option<&'static str>, num_children: usize, has_attributes: bool) -> Template {
    if let Some(template) = TEMPLATES.read().as_ref().and_then(|m| m.get(&(tag, namespace, num_children, has_attributes))) {
        return *template;
    }

    let mut write = TEMPLATES.write();
    let map = write.get_or_insert_with(HashMap::new);
    if let Some(template) = map.get(&(tag, namespace, num_children, has_attributes)) {
        return *template;
    }

    let template = create_template(tag, namespace, num_children, has_attributes);
    map.insert((tag, namespace, num_children, has_attributes), template);
    template
}

fn create_template(tag: &'static str, namespace: Option<&'static str>, num_children: usize, has_attributes: bool) -> Template {
    let mut children_list = Vec::with_capacity(num_children);
    let mut node_paths = Vec::with_capacity(num_children);

    // Root element is at path [0]. Children are at [0, i].
    for i in 0..num_children {
        children_list.push(TemplateNode::Dynamic { id: i });
        let path: &'static [u8] = Box::leak(Box::new([0u8, i as u8]));
        node_paths.push(path);
    }

    let children: &'static [TemplateNode] = Box::leak(children_list.into_boxed_slice());
    let node_paths: &'static [&'static [u8]] = Box::leak(node_paths.into_boxed_slice());

    let mut attrs_list = Vec::with_capacity(1);
    if has_attributes {
        attrs_list.push(TemplateAttribute::Dynamic { id: 0 });
    }
    let attrs: &'static [TemplateAttribute] = Box::leak(attrs_list.into_boxed_slice());

    let roots: &'static [TemplateNode] = Box::leak(Box::new([TemplateNode::Element {
        tag,
        namespace,
        attrs,
        children,
    }]));

    let attr_paths: &'static [&'static [u8]] = if has_attributes {
        Box::leak(Box::new([
            Box::leak(Box::new([0u8])) as &'static [u8]
        ]))
    } else {
        &[]
    };

    Template {
        roots,
        node_paths,
        attr_paths,
    }
}

/// A builder for constructing HTML elements with a fluent API.
///
/// # Example
///
/// ```rust,ignore
/// div()
///     .class("my-class")
///     .id("my-id")
///     .onclick(|_| {})
///     .child("Hello!")
///     .build()
/// ```
#[derive(Default)]
pub struct ElementBuilder {
    tag: &'static str,
    namespace: Option<&'static str>,
    attributes: Vec<Attribute>,
    children: Vec<DynamicNode>,
}

impl ElementBuilder {
    /// Create a new ElementBuilder for the given tag.
    pub fn new(tag: &'static str) -> Self {
        Self {
            tag,
            namespace: None,
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Create a new ElementBuilder with a namespace (e.g., for SVG elements).
    pub fn new_with_namespace(tag: &'static str, namespace: &'static str) -> Self {
        Self {
            tag,
            namespace: Some(namespace),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Add a child element or text node.
    pub fn child(mut self, child: impl IntoDynNode) -> Self {
        self.children.push(child.into_dyn_node());
        self
    }

    /// Add multiple children from an iterator.
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoDynNode>) -> Self {
        for child in children {
            self.children.push(child.into_dyn_node());
        }
        self
    }

    /// Build the element into a VNode (Element).
    pub fn build(self) -> dioxus_core::Element {
        let num_children = self.children.len();
        let has_attributes = !self.attributes.is_empty();
        let template = get_template(self.tag, self.namespace, num_children, has_attributes);

        // Pack each child into a dynamic node
        let dynamic_nodes = self.children.into_boxed_slice();

        // Pack all attributes into a single dynamic attribute group
        let mut dynamic_attrs = Vec::new();
        if has_attributes {
            let mut attributes = self.attributes;
            attributes.sort_by(|a, b| a.name.cmp(b.name));
            dynamic_attrs.push(attributes.into_boxed_slice());
        }
        let dynamic_attrs = dynamic_attrs.into_boxed_slice();

        Ok(VNode::new(None, template, dynamic_nodes, dynamic_attrs))
    }
}

impl IntoDynNode for ElementBuilder {
    fn into_dyn_node(self) -> DynamicNode {
        self.build()
            .map(|v| DynamicNode::Fragment(vec![v]))
            .unwrap_or_default()
    }
}

impl HasAttributes for ElementBuilder {
    fn push_attribute<T>(
        mut self,
        name: &'static str,
        ns: Option<&'static str>,
        attr: impl IntoAttributeValue<T>,
        volatile: bool,
    ) -> Self {
        self.attributes.push(Attribute {
            name,
            namespace: ns,
            value: attr.into_value(),
            volatile,
        });
        self
    }
}

// =============================================================================
// Global Attributes (available on all HTML elements)
// =============================================================================

impl ElementBuilder {
    /// Set the class attribute.
    pub fn class(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("class", None, value, false)
    }

    /// Set the id attribute.
    pub fn id(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("id", None, value, false)
    }

    /// Set the style attribute.
    pub fn style(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("style", None, value, false)
    }

    /// Set the title attribute.
    pub fn title(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("title", None, value, false)
    }

    /// Set the hidden attribute.
    pub fn hidden(self, value: bool) -> Self {
        self.push_attribute("hidden", None, value, false)
    }

    /// Set the tabindex attribute.
    pub fn tabindex(self, value: i32) -> Self {
        self.push_attribute("tabindex", None, value, false)
    }

    /// Set the role attribute (ARIA).
    pub fn role(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("role", None, value, false)
    }

    /// Set the draggable attribute.
    pub fn draggable(self, value: bool) -> Self {
        self.push_attribute("draggable", None, value.to_string(), false)
    }

    /// Set a data-* attribute.
    pub fn data(self, name: &'static str, value: impl IntoAttributeValue) -> Self {
        // Note: For now, we use a static name. A more dynamic approach would need changes.
        self.push_attribute(name, None, value, false)
    }
}

// =============================================================================
// Event Handlers
// =============================================================================

impl ElementBuilder {
    /// Set the onclick handler.
    pub fn onclick(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onclick(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the ondblclick handler.
    pub fn ondblclick(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        #[allow(deprecated)]
        let attr = dioxus_html::ondblclick(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmousedown handler.
    pub fn onmousedown(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmousedown(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmouseup handler.
    pub fn onmouseup(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmouseup(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmouseover handler.
    pub fn onmouseover(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmouseover(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmousemove handler.
    pub fn onmousemove(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmousemove(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmouseout handler.
    pub fn onmouseout(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmouseout(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmouseenter handler.
    pub fn onmouseenter(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmouseenter(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onmouseleave handler.
    pub fn onmouseleave(self, handler: impl FnMut(dioxus_core::Event<MouseData>) + 'static) -> Self {
        let attr = dioxus_html::onmouseleave(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onkeydown handler.
    pub fn onkeydown(self, handler: impl FnMut(dioxus_core::Event<KeyboardData>) + 'static) -> Self {
        let attr = dioxus_html::onkeydown(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onkeyup handler.
    pub fn onkeyup(self, handler: impl FnMut(dioxus_core::Event<KeyboardData>) + 'static) -> Self {
        let attr = dioxus_html::onkeyup(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onkeypress handler.
    pub fn onkeypress(
        self,
        handler: impl FnMut(dioxus_core::Event<KeyboardData>) + 'static,
    ) -> Self {
        let attr = dioxus_html::onkeypress(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onfocus handler.
    pub fn onfocus(self, handler: impl FnMut(dioxus_core::Event<FocusData>) + 'static) -> Self {
        let attr = dioxus_html::onfocus(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onblur handler.
    pub fn onblur(self, handler: impl FnMut(dioxus_core::Event<FocusData>) + 'static) -> Self {
        let attr = dioxus_html::onblur(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }
    /// Set the oninput handler.
    pub fn oninput(self, handler: impl FnMut(dioxus_core::Event<FormData>) + 'static) -> Self {
        let attr = dioxus_html::oninput(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onchange handler.
    pub fn onchange(self, handler: impl FnMut(dioxus_core::Event<FormData>) + 'static) -> Self {
        let attr = dioxus_html::onchange(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }

    /// Set the onsubmit handler.
    pub fn onsubmit(self, handler: impl FnMut(dioxus_core::Event<FormData>) + 'static) -> Self {
        let attr = dioxus_html::onsubmit(handler);
        self.push_attribute(attr.name, attr.namespace, attr.value, attr.volatile)
    }
}

// =============================================================================
// Form Element Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the disabled attribute.
    pub fn disabled(self, value: bool) -> Self {
        self.push_attribute("disabled", None, value, false)
    }

    /// Set the value attribute.
    pub fn value(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("value", None, value, true) // volatile for controlled inputs
    }

    /// Set the placeholder attribute.
    pub fn placeholder(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("placeholder", None, value, false)
    }

    /// Set the name attribute.
    pub fn name(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("name", None, value, false)
    }

    /// Set the type attribute (for input elements).
    pub fn r#type(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("type", None, value, false)
    }

    /// Set the checked attribute (for checkboxes/radios).
    pub fn checked(self, value: bool) -> Self {
        self.push_attribute("checked", None, value, true) // volatile for controlled inputs
    }

    /// Set the readonly attribute.
    pub fn readonly(self, value: bool) -> Self {
        self.push_attribute("readonly", None, value, false)
    }

    /// Set the required attribute.
    pub fn required(self, value: bool) -> Self {
        self.push_attribute("required", None, value, false)
    }

    /// Set the maxlength attribute.
    pub fn maxlength(self, value: i32) -> Self {
        self.push_attribute("maxlength", None, value, false)
    }

    /// Set the minlength attribute.
    pub fn minlength(self, value: i32) -> Self {
        self.push_attribute("minlength", None, value, false)
    }

    /// Set the min attribute.
    pub fn min(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("min", None, value, false)
    }

    /// Set the max attribute.
    pub fn max(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("max", None, value, false)
    }

    /// Set the step attribute.
    pub fn step(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("step", None, value, false)
    }

    /// Set the autocomplete attribute.
    pub fn autocomplete(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("autocomplete", None, value, false)
    }
}

// =============================================================================
// Link/Anchor Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the href attribute.
    pub fn href(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("href", None, value, false)
    }

    /// Set the target attribute.
    pub fn target(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("target", None, value, false)
    }

    /// Set the rel attribute.
    pub fn rel(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("rel", None, value, false)
    }

    /// Set the download attribute.
    pub fn download(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("download", None, value, false)
    }
}

// =============================================================================
// Image/Media Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the src attribute.
    pub fn src(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("src", None, value, false)
    }

    /// Set the alt attribute.
    pub fn alt(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("alt", None, value, false)
    }

    /// Set the width attribute.
    pub fn width(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("width", None, value, false)
    }

    /// Set the height attribute.
    pub fn height(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("height", None, value, false)
    }

    /// Set the loading attribute (lazy/eager).
    pub fn loading(self, value: impl IntoAttributeValue) -> Self {
        self.push_attribute("loading", None, value, false)
    }
}

// =============================================================================
// Table Attributes
// =============================================================================

impl ElementBuilder {
    /// Set the colspan attribute.
    pub fn colspan(self, value: i32) -> Self {
        self.push_attribute("colspan", None, value, false)
    }

    /// Set the rowspan attribute.
    pub fn rowspan(self, value: i32) -> Self {
        self.push_attribute("rowspan", None, value, false)
    }
}

// =============================================================================
// Element Constructor Functions
// =============================================================================

// Document Metadata
pub fn head() -> ElementBuilder { ElementBuilder::new("head") }
pub fn title() -> ElementBuilder { ElementBuilder::new("title") }
pub fn base() -> ElementBuilder { ElementBuilder::new("base") }
pub fn link() -> ElementBuilder { ElementBuilder::new("link") }
pub fn meta() -> ElementBuilder { ElementBuilder::new("meta") }
pub fn style() -> ElementBuilder { ElementBuilder::new("style") }

// Sectioning Root
pub fn body() -> ElementBuilder { ElementBuilder::new("body") }

// Content Sectioning
pub fn address() -> ElementBuilder { ElementBuilder::new("address") }
pub fn article() -> ElementBuilder { ElementBuilder::new("article") }
pub fn aside() -> ElementBuilder { ElementBuilder::new("aside") }
pub fn footer() -> ElementBuilder { ElementBuilder::new("footer") }
pub fn header() -> ElementBuilder { ElementBuilder::new("header") }
pub fn h1() -> ElementBuilder { ElementBuilder::new("h1") }
pub fn h2() -> ElementBuilder { ElementBuilder::new("h2") }
pub fn h3() -> ElementBuilder { ElementBuilder::new("h3") }
pub fn h4() -> ElementBuilder { ElementBuilder::new("h4") }
pub fn h5() -> ElementBuilder { ElementBuilder::new("h5") }
pub fn h6() -> ElementBuilder { ElementBuilder::new("h6") }
pub fn main() -> ElementBuilder { ElementBuilder::new("main") }
pub fn nav() -> ElementBuilder { ElementBuilder::new("nav") }
pub fn section() -> ElementBuilder { ElementBuilder::new("section") }
pub fn hgroup() -> ElementBuilder { ElementBuilder::new("hgroup") }

// Text Content
pub fn blockquote() -> ElementBuilder { ElementBuilder::new("blockquote") }
pub fn dd() -> ElementBuilder { ElementBuilder::new("dd") }
pub fn div() -> ElementBuilder { ElementBuilder::new("div") }
pub fn dl() -> ElementBuilder { ElementBuilder::new("dl") }
pub fn dt() -> ElementBuilder { ElementBuilder::new("dt") }
pub fn figcaption() -> ElementBuilder { ElementBuilder::new("figcaption") }
pub fn figure() -> ElementBuilder { ElementBuilder::new("figure") }
pub fn hr() -> ElementBuilder { ElementBuilder::new("hr") }
pub fn li() -> ElementBuilder { ElementBuilder::new("li") }
pub fn ol() -> ElementBuilder { ElementBuilder::new("ol") }
pub fn p() -> ElementBuilder { ElementBuilder::new("p") }
pub fn pre() -> ElementBuilder { ElementBuilder::new("pre") }
pub fn ul() -> ElementBuilder { ElementBuilder::new("ul") }
pub fn menu() -> ElementBuilder { ElementBuilder::new("menu") }

// Inline Text Semantics
pub fn a() -> ElementBuilder { ElementBuilder::new("a") }
pub fn abbr() -> ElementBuilder { ElementBuilder::new("abbr") }
pub fn b() -> ElementBuilder { ElementBuilder::new("b") }
pub fn bdi() -> ElementBuilder { ElementBuilder::new("bdi") }
pub fn bdo() -> ElementBuilder { ElementBuilder::new("bdo") }
pub fn br() -> ElementBuilder { ElementBuilder::new("br") }
pub fn cite() -> ElementBuilder { ElementBuilder::new("cite") }
pub fn code() -> ElementBuilder { ElementBuilder::new("code") }
pub fn data() -> ElementBuilder { ElementBuilder::new("data") }
pub fn dfn() -> ElementBuilder { ElementBuilder::new("dfn") }
pub fn em() -> ElementBuilder { ElementBuilder::new("em") }
pub fn i() -> ElementBuilder { ElementBuilder::new("i") }
pub fn kbd() -> ElementBuilder { ElementBuilder::new("kbd") }
pub fn mark() -> ElementBuilder { ElementBuilder::new("mark") }
pub fn q() -> ElementBuilder { ElementBuilder::new("q") }
pub fn rp() -> ElementBuilder { ElementBuilder::new("rp") }
pub fn rt() -> ElementBuilder { ElementBuilder::new("rt") }
pub fn ruby() -> ElementBuilder { ElementBuilder::new("ruby") }
pub fn s() -> ElementBuilder { ElementBuilder::new("s") }
pub fn samp() -> ElementBuilder { ElementBuilder::new("samp") }
pub fn small() -> ElementBuilder { ElementBuilder::new("small") }
pub fn span() -> ElementBuilder { ElementBuilder::new("span") }
pub fn strong() -> ElementBuilder { ElementBuilder::new("strong") }
pub fn sub() -> ElementBuilder { ElementBuilder::new("sub") }
pub fn sup() -> ElementBuilder { ElementBuilder::new("sup") }
pub fn time() -> ElementBuilder { ElementBuilder::new("time") }
pub fn u() -> ElementBuilder { ElementBuilder::new("u") }
pub fn var() -> ElementBuilder { ElementBuilder::new("var") }
pub fn wbr() -> ElementBuilder { ElementBuilder::new("wbr") }

// Image and Multimedia
pub fn area() -> ElementBuilder { ElementBuilder::new("area") }
pub fn audio() -> ElementBuilder { ElementBuilder::new("audio") }
pub fn img() -> ElementBuilder { ElementBuilder::new("img") }
pub fn map() -> ElementBuilder { ElementBuilder::new("map") }
pub fn track() -> ElementBuilder { ElementBuilder::new("track") }
pub fn video() -> ElementBuilder { ElementBuilder::new("video") }

// Embedded Content
pub fn embed() -> ElementBuilder { ElementBuilder::new("embed") }
pub fn iframe() -> ElementBuilder { ElementBuilder::new("iframe") }
pub fn object() -> ElementBuilder { ElementBuilder::new("object") }
pub fn param() -> ElementBuilder { ElementBuilder::new("param") }
pub fn picture() -> ElementBuilder { ElementBuilder::new("picture") }
pub fn portal() -> ElementBuilder { ElementBuilder::new("portal") }
pub fn source() -> ElementBuilder { ElementBuilder::new("source") }

// SVG and MathML (with namespace)
pub fn svg() -> ElementBuilder { ElementBuilder::new_with_namespace("svg", "http://www.w3.org/2000/svg") }
pub fn math() -> ElementBuilder { ElementBuilder::new_with_namespace("math", "http://www.w3.org/1998/Math/MathML") }

// Scripting
pub fn canvas() -> ElementBuilder { ElementBuilder::new("canvas") }
pub fn noscript() -> ElementBuilder { ElementBuilder::new("noscript") }
pub fn script() -> ElementBuilder { ElementBuilder::new("script") }

// Demarcating Edits
pub fn del() -> ElementBuilder { ElementBuilder::new("del") }
pub fn ins() -> ElementBuilder { ElementBuilder::new("ins") }

// Table Content
pub fn caption() -> ElementBuilder { ElementBuilder::new("caption") }
pub fn col() -> ElementBuilder { ElementBuilder::new("col") }
pub fn colgroup() -> ElementBuilder { ElementBuilder::new("colgroup") }
pub fn table() -> ElementBuilder { ElementBuilder::new("table") }
pub fn tbody() -> ElementBuilder { ElementBuilder::new("tbody") }
pub fn td() -> ElementBuilder { ElementBuilder::new("td") }
pub fn tfoot() -> ElementBuilder { ElementBuilder::new("tfoot") }
pub fn th() -> ElementBuilder { ElementBuilder::new("th") }
pub fn thead() -> ElementBuilder { ElementBuilder::new("thead") }
pub fn tr() -> ElementBuilder { ElementBuilder::new("tr") }

// Forms
pub fn button() -> ElementBuilder { ElementBuilder::new("button") }
pub fn datalist() -> ElementBuilder { ElementBuilder::new("datalist") }
pub fn fieldset() -> ElementBuilder { ElementBuilder::new("fieldset") }
pub fn form() -> ElementBuilder { ElementBuilder::new("form") }
pub fn input() -> ElementBuilder { ElementBuilder::new("input") }
pub fn label() -> ElementBuilder { ElementBuilder::new("label") }
pub fn legend() -> ElementBuilder { ElementBuilder::new("legend") }
pub fn meter() -> ElementBuilder { ElementBuilder::new("meter") }
pub fn optgroup() -> ElementBuilder { ElementBuilder::new("optgroup") }
pub fn option() -> ElementBuilder { ElementBuilder::new("option") }
pub fn output() -> ElementBuilder { ElementBuilder::new("output") }
pub fn progress() -> ElementBuilder { ElementBuilder::new("progress") }
pub fn select() -> ElementBuilder { ElementBuilder::new("select") }
pub fn textarea() -> ElementBuilder { ElementBuilder::new("textarea") }

// Interactive Elements
pub fn details() -> ElementBuilder { ElementBuilder::new("details") }
pub fn dialog() -> ElementBuilder { ElementBuilder::new("dialog") }
pub fn summary() -> ElementBuilder { ElementBuilder::new("summary") }

// Web Components
pub fn slot() -> ElementBuilder { ElementBuilder::new("slot") }
pub fn template() -> ElementBuilder { ElementBuilder::new("template") }
