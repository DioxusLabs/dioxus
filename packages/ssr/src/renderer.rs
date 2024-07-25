use super::cache::Segment;
use crate::cache::StringCache;

use dioxus_core::{prelude::*, AttributeValue, DynamicNode};
use rustc_hash::FxHashMap;
use std::fmt::Write;
use std::sync::Arc;

type ComponentRenderCallback = Arc<
    dyn Fn(&mut Renderer, &mut dyn Write, &VirtualDom, ScopeId) -> std::fmt::Result + Send + Sync,
>;

/// A virtualdom renderer that caches the templates it has seen for faster rendering
#[derive(Default)]
pub struct Renderer {
    /// Choose to write ElementIDs into elements so the page can be re-hydrated later on
    pub pre_render: bool,

    /// A callback used to render components. You can set this callback to control what components are rendered and add wrappers around components that are not present in CSR
    render_components: Option<ComponentRenderCallback>,

    /// A cache of templates that have been rendered
    template_cache: FxHashMap<usize, Arc<StringCache>>,

    /// The current dynamic node id for hydration
    dynamic_node_id: usize,
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
        self.reset_hydration();
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

    /// Reset the renderer hydration state
    pub fn reset_hydration(&mut self) {
        self.dynamic_node_id = 0;
    }

    pub fn render_scope<W: Write + ?Sized>(
        &mut self,
        buf: &mut W,
        dom: &VirtualDom,
        scope: ScopeId,
    ) -> std::fmt::Result {
        let node = dom.get_scope(scope).unwrap().root_node();
        self.render_template(buf, dom, node)?;

        Ok(())
    }

    fn render_template<W: Write + ?Sized>(
        &mut self,
        mut buf: &mut W,
        dom: &VirtualDom,
        template: &VNode,
    ) -> std::fmt::Result {
        let entry = self
            .template_cache
            .entry(template.template.get().id())
            .or_insert_with(move || Arc::new(StringCache::from_template(template).unwrap()))
            .clone();

        let mut inner_html = None;

        // We need to keep track of the dynamic styles so we can insert them into the right place
        let mut accumulated_dynamic_styles = Vec::new();

        // We need to keep track of the listeners so we can insert them into the right place
        let mut accumulated_listeners = Vec::new();

        // We keep track of the index we are on manually so that we can jump forward to a new section quickly without iterating every item
        let mut index = 0;

        while let Some(segment) = entry.segments.get(index) {
            match segment {
                Segment::HydrationOnlySection(jump_to) => {
                    // If we are not prerendering, we don't need to write the content of the hydration only section
                    // Instead we can jump to the next section
                    if !self.pre_render {
                        index = *jump_to;
                        continue;
                    }
                }
                Segment::Attr(idx) => {
                    let attrs = &*template.dynamic_attrs[*idx];
                    for attr in attrs {
                        if attr.name == "dangerous_inner_html" {
                            inner_html = Some(attr);
                        } else if attr.namespace == Some("style") {
                            accumulated_dynamic_styles.push(attr);
                        } else if BOOL_ATTRS.contains(&attr.name) {
                            if truthy(&attr.value) {
                                write_attribute(buf, attr)?;
                            }
                        } else {
                            write_attribute(buf, attr)?;
                        }

                        if self.pre_render {
                            if let AttributeValue::Listener(_) = &attr.value {
                                // The onmounted event doesn't need a DOM listener
                                if attr.name != "onmounted" {
                                    accumulated_listeners.push(attr.name);
                                }
                            }
                        }
                    }
                }
                Segment::Node(idx) => match &template.dynamic_nodes[*idx] {
                    DynamicNode::Component(node) => {
                        if let Some(render_components) = self.render_components.clone() {
                            let scope_id = node.mounted_scope_id(*idx, template, dom).unwrap();

                            render_components(self, &mut buf, dom, scope_id)?;
                        } else {
                            let scope = node.mounted_scope(*idx, template, dom).unwrap();
                            let node = scope.root_node();
                            self.render_template(buf, dom, node)?
                        }
                    }
                    DynamicNode::Text(text) => {
                        // in SSR, we are concerned that we can't hunt down the right text node since they might get merged
                        if self.pre_render {
                            write!(buf, "<!--node-id{}-->", self.dynamic_node_id)?;
                            self.dynamic_node_id += 1;
                        }

                        write!(
                            buf,
                            "{}",
                            askama_escape::escape(&text.value, askama_escape::Html)
                        )?;

                        if self.pre_render {
                            write!(buf, "<!--#-->")?;
                        }
                    }
                    DynamicNode::Fragment(nodes) => {
                        for child in nodes {
                            self.render_template(buf, dom, child)?;
                        }
                    }

                    DynamicNode::Placeholder(_) => {
                        if self.pre_render {
                            write!(buf, "<!--placeholder{}-->", self.dynamic_node_id)?;
                            self.dynamic_node_id += 1;
                        }
                    }
                },

                Segment::PreRendered(contents) => write!(buf, "{contents}")?,

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

                Segment::AttributeNodeMarker => {
                    // first write the id
                    write!(buf, "{}", self.dynamic_node_id)?;
                    self.dynamic_node_id += 1;
                    // then write any listeners
                    for name in accumulated_listeners.drain(..) {
                        write!(buf, ",{}:", &name[2..])?;
                        write!(buf, "{}", dioxus_html::event_bubbles(&name[2..]) as u8)?;
                    }
                }

                Segment::RootNodeMarker => {
                    write!(buf, "{}", self.dynamic_node_id)?;
                    self.dynamic_node_id += 1
                }
            }

            index += 1;
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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    for item in renderer.template_cache.iter() {
        if item.1.segments.len() > 10 {
            assert_eq!(
                item.1.segments,
                vec![
                    PreRendered("<div class=\"asdasdasd asdasdasd\"".to_string()),
                    Attr(0),
                    StyleMarker {
                        inside_style_tag: false
                    },
                    HydrationOnlySection(7), // jump to `>` if we don't need to hydrate
                    PreRendered(" data-node-hydration=\"".to_string()),
                    AttributeNodeMarker,
                    PreRendered("\"".to_string()),
                    PreRendered(">".to_string()),
                    InnerHtmlMarker,
                    PreRendered("Hello world 1 --&gt;".to_string()),
                    Node(0),
                    PreRendered(
                        "&lt;-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div>"
                            .to_string()
                    ),
                    Node(1),
                    Node(2),
                    PreRendered("</div>".to_string())
                ]
            );
        }
    }

    use Segment::*;

    assert_eq!(out, "<div class=\"asdasdasd asdasdasd\" id=\"id-123\">Hello world 1 --&gt;123&lt;-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div>&lt;/diiiiiiiiv&gt;<div>finalize 0</div><div>finalize 1</div><div>finalize 2</div><div>finalize 3</div><div>finalize 4</div></div>");
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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    for item in renderer.template_cache.iter() {
        if item.1.segments.len() > 5 {
            assert_eq!(
                item.1.segments,
                vec![
                    PreRendered("<div class=\"asdasdasd\"".to_string()),
                    HydrationOnlySection(5), // jump to `>` if we don't need to hydrate
                    PreRendered(" data-node-hydration=\"".to_string()),
                    RootNodeMarker,
                    PreRendered("\"".to_string()),
                    PreRendered(">".to_string()),
                    Node(0),
                    PreRendered("</div>".to_string())
                ]
            );
        }
    }

    use Segment::*;

    assert_eq!(out, "<div class=\"asdasdasd\"></div>");
}

#[test]
fn empty_render_works() {
    use dioxus::prelude::*;

    fn app() -> Element {
        rsx! {}
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    for item in renderer.template_cache.iter() {
        if item.1.segments.len() > 5 {
            assert_eq!(item.1.segments, vec![]);
        }
    }
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
        AttributeValue::Text(value) => write!(buf, " {name}=\"{value}\""),
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
        AttributeValue::Text(value) => write!(buf, "{}", value),
        AttributeValue::Bool(value) => write!(buf, "{}", value),
        AttributeValue::Int(value) => write!(buf, "{}", value),
        AttributeValue::Float(value) => write!(buf, "{}", value),
        _ => Ok(()),
    }
}
