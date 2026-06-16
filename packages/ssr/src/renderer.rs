use super::cache::Segment;
use crate::cache::StringCache;

use dioxus_core::{
    Attribute, AttributeValue, DynamicNode, Element, MountedVNode, ScopeId, Template, VirtualDom,
};
use rustc_hash::FxHashMap;
use std::fmt::Write;
use std::sync::Arc;

type ComponentRenderCallback = Arc<
    dyn Fn(&mut Renderer, &mut dyn Write, &VirtualDom, ScopeId) -> std::fmt::Result + Send + Sync,
>;

/// A virtualdom renderer that caches the templates it has seen for faster rendering
#[derive(Default)]
pub struct Renderer {
    /// A callback used to render components. You can set this callback to control what components are rendered and add wrappers around components that are not present in CSR
    render_components: Option<ComponentRenderCallback>,

    /// A cache of templates that have been rendered
    template_cache: FxHashMap<Template, Arc<StringCache>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the callback that the renderer uses to render components
    pub fn set_render_components(
        &mut self,
        callback: impl Fn(&mut Renderer, &mut dyn Write, &VirtualDom, ScopeId) -> std::fmt::Result
        + Send
        + Sync
        + 'static,
    ) {
        self.render_components = Some(Arc::new(callback));
    }

    /// Completely clear the renderer cache.
    pub fn clear(&mut self) {
        self.template_cache.clear();
        self.render_components = None;
    }

    /// Reset the callback that the renderer uses to render components
    pub fn reset_render_components(&mut self) {
        self.render_components = None;
    }

    pub fn render(&mut self, dom: &VirtualDom) -> String {
        let mut buf = String::new();
        self.render_to(&mut buf, dom).unwrap();
        buf
    }

    pub fn render_to<W: Write + ?Sized>(
        &mut self,
        buf: &mut W,
        dom: &VirtualDom,
    ) -> std::fmt::Result {
        self.render_scope(buf, dom, ScopeId::ROOT)
    }

    /// Render an element to a string
    pub fn render_element(&mut self, element: Element) -> String {
        let mut buf = String::new();
        self.render_element_to(&mut buf, element).unwrap();
        buf
    }

    /// Render an element to the buffer
    pub fn render_element_to<W: Write + ?Sized>(
        &mut self,
        buf: &mut W,
        element: Element,
    ) -> std::fmt::Result {
        fn lazy_app(props: Element) -> Element {
            props
        }
        let mut dom = VirtualDom::new_with_props(lazy_app, element);
        dom.rebuild_in_place();
        self.render_to(buf, &dom)
    }

    pub fn render_scope<W: Write + ?Sized>(
        &mut self,
        buf: &mut W,
        dom: &VirtualDom,
        scope: ScopeId,
    ) -> std::fmt::Result {
        let node = dom
            .get_scope(scope)
            .unwrap()
            .try_mounted_root_node()
            .unwrap();
        self.render_template(buf, dom, node, true)?;

        Ok(())
    }

    fn render_template<W: Write + ?Sized>(
        &mut self,
        mut buf: &mut W,
        dom: &VirtualDom,
        template: MountedVNode<'_>,
        parent_escaped: bool,
    ) -> std::fmt::Result {
        let entry = self
            .template_cache
            .entry(template.vnode().template)
            .or_insert_with(move || Arc::new(StringCache::from_template(template.vnode()).unwrap()))
            .clone();

        let mut inner_html = None;

        // We need to keep track of the dynamic styles so we can insert them into the right place
        let mut accumulated_dynamic_styles = Vec::new();

        for segment in entry.segments.iter() {
            match segment {
                Segment::Attr(idx) => {
                    let attrs = template.dynamic_values[*idx]
                        .as_attrs()
                        .expect("SSR attr segment must point at dynamic attributes");
                    for attr in attrs {
                        if attr.name == "dangerous_inner_html" {
                            inner_html = Some(attr);
                        } else if attr.namespace == Some("style") {
                            accumulated_dynamic_styles.push(attr);
                        } else if BOOL_ATTRS.contains(&attr.name) {
                            if truthy(&attr.value) {
                                write_attribute(buf, attr)?;
                            }
                        } else if !matches!(attr.value, AttributeValue::Listener(_)) {
                            write_attribute(buf, attr)?;
                        }
                    }
                }
                Segment::Node { index, escape_text } => {
                    let escaped = escape_text.should_escape(parent_escaped);
                    match template.dynamic_values[*index]
                        .as_node()
                        .expect("SSR node segment must point at a dynamic node")
                    {
                        DynamicNode::Component(node) => {
                            if let Some(render_components) = self.render_components.clone() {
                                let scope_id =
                                    node.mounted_scope_id(*index, template, dom).unwrap();

                                render_components(self, &mut buf, dom, scope_id)?;
                            } else {
                                let scope = node.mounted_scope(*index, template, dom).unwrap();
                                let node = scope.try_mounted_root_node().unwrap();
                                self.render_template(buf, dom, node, escaped)?
                            }
                        }
                        DynamicNode::Text(text) => {
                            if escaped {
                                write!(
                                    buf,
                                    "{}",
                                    askama_escape::escape(&text.value, askama_escape::Html)
                                )?;
                            } else {
                                write!(buf, "{}", text.value)?;
                            }
                        }
                        DynamicNode::Fragment(nodes) => {
                            // An empty fragment contributes no HTML — the web hydrator handles
                            // the position via the markerless walk script.
                            let mounted_children = template.mounted_fragment_children(*index, dom);
                            assert_eq!(
                                mounted_children.len(),
                                nodes.len(),
                                "fragment dynamic node {index} is not mounted"
                            );

                            for child in mounted_children {
                                self.render_template(buf, dom, child, escaped)?;
                            }
                        }
                    }
                }

                Segment::PreRendered(contents) => write!(buf, "{contents}")?,
                Segment::PreRenderedMaybeEscaped {
                    value,
                    renderer_if_escaped,
                } => {
                    if *renderer_if_escaped == parent_escaped {
                        write!(buf, "{value}")?;
                    }
                }

                Segment::StyleMarker { inside_style_tag } => {
                    if !accumulated_dynamic_styles.is_empty() {
                        // if we are inside a style tag, we don't need to write the style attribute
                        if !*inside_style_tag {
                            write!(buf, " style=\"")?;
                        }
                        for attr in &accumulated_dynamic_styles {
                            write!(buf, "{}:", attr.name)?;
                            write_value_unquoted(buf, &attr.value)?;
                            write!(buf, ";")?;
                        }
                        if !*inside_style_tag {
                            write!(buf, "\"")?;
                        }

                        // clear the accumulated styles
                        accumulated_dynamic_styles.clear();
                    }
                }

                Segment::InnerHtmlMarker => {
                    if let Some(inner_html) = inner_html.take() {
                        let inner_html = &inner_html.value;
                        match inner_html {
                            AttributeValue::Text(value) => write!(buf, "{}", value)?,
                            AttributeValue::Bool(value) => write!(buf, "{}", value)?,
                            AttributeValue::Float(f) => write!(buf, "{}", f)?,
                            AttributeValue::Int(i) => write!(buf, "{}", i)?,
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[test]
fn to_string_works() {
    use dioxus::prelude::*;

    fn app() -> Element {
        let dynamic = 123;
        let dyn2 = "</diiiiiiiiv>"; // this should be escaped

        rsx! {
            div { class: "asdasdasd", class: "asdasdasd", id: "id-{dynamic}",
                "Hello world 1 -->"
                "{dynamic}"
                "<-- Hello world 2"
                div { "nest 1" }
                div {}
                div { "nest 2" }
                "{dyn2}"
                for i in (0..5) {
                    div { "finalize {i}" }
                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    assert_eq!(
        out,
        "<div class=\"asdasdasd asdasdasd\" id=\"id-123\">Hello world 1 --&#62;123&#60;-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div>&#60;/diiiiiiiiv&#62;<div>finalize 0</div><div>finalize 1</div><div>finalize 2</div><div>finalize 3</div><div>finalize 4</div></div>"
    );
}

#[test]
fn empty_for_loop_works() {
    use dioxus::prelude::*;

    fn app() -> Element {
        rsx! {
            div { class: "asdasdasd",
                for _ in (0..5) {

                }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    assert_eq!(out, "<div class=\"asdasdasd\"></div>");
}

#[test]
fn empty_render_works() {
    use dioxus::prelude::*;

    fn app() -> Element {
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);
    assert_eq!(out, "");
}

pub(crate) const BOOL_ATTRS: &[&str] = &[
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
    "inert",
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

pub(crate) fn str_truthy(value: &str) -> bool {
    !value.is_empty() && value != "0" && value.to_lowercase() != "false"
}

pub(crate) fn truthy(value: &AttributeValue) -> bool {
    match value {
        AttributeValue::Text(value) => str_truthy(value),
        AttributeValue::Bool(value) => *value,
        AttributeValue::Int(value) => *value != 0,
        AttributeValue::Float(value) => *value != 0.0,
        _ => false,
    }
}

pub(crate) fn write_attribute<W: Write + ?Sized>(
    buf: &mut W,
    attr: &Attribute,
) -> std::fmt::Result {
    let name = &attr.name;
    match &attr.value {
        AttributeValue::Text(value) => write!(
            buf,
            " {name}=\"{}\"",
            askama_escape::escape(value, askama_escape::Html)
        ),
        AttributeValue::Bool(value) => write!(buf, " {name}={value}"),
        AttributeValue::Int(value) => write!(buf, " {name}={value}"),
        AttributeValue::Float(value) => write!(buf, " {name}={value}"),
        _ => Ok(()),
    }
}

pub(crate) fn write_value_unquoted<W: Write + ?Sized>(
    buf: &mut W,
    value: &AttributeValue,
) -> std::fmt::Result {
    match value {
        AttributeValue::Text(value) => {
            write!(buf, "{}", askama_escape::escape(value, askama_escape::Html))
        }
        AttributeValue::Bool(value) => write!(buf, "{}", value),
        AttributeValue::Int(value) => write!(buf, "{}", value),
        AttributeValue::Float(value) => write!(buf, "{}", value),
        _ => Ok(()),
    }
}
