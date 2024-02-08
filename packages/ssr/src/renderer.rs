use super::cache::Segment;
use crate::cache::StringCache;
use dioxus_core::RenderReturn;

use dioxus_core::Attribute;
use dioxus_core::{prelude::*, AttributeValue, DynamicNode};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;

/// A virtualdom renderer that caches the templates it has seen for faster rendering
#[derive(Default)]
pub struct Renderer {
    /// should we do our best to prettify the output?
    pub pretty: bool,

    /// Control if elements are written onto a new line
    pub newline: bool,

    /// Should we sanitize text nodes? (escape HTML)
    pub sanitize: bool,

    /// Choose to write ElementIDs into elements so the page can be re-hydrated later on
    pub pre_render: bool,

    // Currently not implemented
    // Don't proceed onto new components. Instead, put the name of the component.
    pub skip_components: bool,

    /// A cache of templates that have been rendered
    template_cache: HashMap<&'static str, Arc<StringCache>>,

    /// The current dynamic node id for hydration
    dynamic_node_id: usize,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self, dom: &VirtualDom) -> String {
        let mut buf = String::new();
        self.render_to(&mut buf, dom).unwrap();
        buf
    }

    pub fn render_to(&mut self, buf: &mut impl Write, dom: &VirtualDom) -> std::fmt::Result {
        self.render_scope(buf, dom, ScopeId::ROOT)
    }

    pub fn render_scope(
        &mut self,
        buf: &mut impl Write,
        dom: &VirtualDom,
        scope: ScopeId,
    ) -> std::fmt::Result {
        // We should never ever run into async or errored nodes in SSR
        // Error boundaries and suspense boundaries will convert these to sync
        if let RenderReturn::Ready(node) = dom.get_scope(scope).unwrap().root_node() {
            self.dynamic_node_id = 0;
            self.render_template(buf, dom, node)?
        };

        Ok(())
    }

    fn render_template(
        &mut self,
        buf: &mut impl Write,
        dom: &VirtualDom,
        template: &VNode,
    ) -> std::fmt::Result {
        let entry = self
            .template_cache
            .entry(template.template.get().name)
            .or_insert_with({
                let prerender = self.pre_render;
                move || Arc::new(StringCache::from_template(template, prerender).unwrap())
            })
            .clone();

        let mut inner_html = None;

        // We need to keep track of the dynamic styles so we can insert them into the right place
        let mut accumulated_dynamic_styles = Vec::new();

        // We need to keep track of the listeners so we can insert them into the right place
        let mut accumulated_listeners = Vec::new();

        for segment in entry.segments.iter() {
            match segment {
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
                        if self.skip_components {
                            write!(buf, "<{}><{}/>", node.name, node.name)?;
                        } else {
                            let scope = node.mounted_scope(*idx, template, dom).unwrap();
                            let node = scope.root_node();
                            match node {
                                RenderReturn::Ready(node) => {
                                    self.render_template(buf, dom, node)?
                                }
                                _ => todo!(
                                    "generally, scopes should be sync, only if being traversed"
                                ),
                            }
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
                            write!(
                                buf,
                                "<pre data-node-hydration={}></pre>",
                                self.dynamic_node_id
                            )?;
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
                        write!(buf, "{}", dioxus_html::event_bubbles(name) as u8)?;
                    }
                }

                Segment::RootNodeMarker => {
                    write!(buf, "{}", self.dynamic_node_id)?;
                    self.dynamic_node_id += 1
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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    for item in renderer.template_cache.iter() {
        if item.1.segments.len() > 5 {
            assert_eq!(
                item.1.segments,
                vec![
                    PreRendered("<div class=\"asdasdasd asdasdasd\"".into(),),
                    Attr(0,),
                    StyleMarker {
                        inside_style_tag: false,
                    },
                    PreRendered(">".into()),
                    InnerHtmlMarker,
                    PreRendered("Hello world 1 --&gt;".into(),),
                    Node(0,),
                    PreRendered(
                        "&lt;-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div>".into(),
                    ),
                    Node(1,),
                    Node(2,),
                    PreRendered("</div>".into(),),
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
                    PreRendered("<div class=\"asdasdasd\"".into(),),
                    Attr(0,),
                    StyleMarker {
                        inside_style_tag: false,
                    },
                    PreRendered(">".into()),
                    InnerHtmlMarker,
                    PreRendered("</div>".into(),),
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

pub(crate) fn write_attribute(buf: &mut impl Write, attr: &Attribute) -> std::fmt::Result {
    let name = &attr.name;
    match &attr.value {
        AttributeValue::Text(value) => write!(buf, " {name}=\"{value}\""),
        AttributeValue::Bool(value) => write!(buf, " {name}={value}"),
        AttributeValue::Int(value) => write!(buf, " {name}={value}"),
        AttributeValue::Float(value) => write!(buf, " {name}={value}"),
        _ => Ok(()),
    }
}

pub(crate) fn write_value_unquoted(
    buf: &mut impl Write,
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
