use super::cache::Segment;
use crate::cache::StringCache;
use dioxus_core::{prelude::*, AttributeValue, DynamicNode, RenderReturn};
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

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
    template_cache: HashMap<&'static str, Rc<StringCache>>,
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
        self.render_scope(buf, dom, ScopeId(0))
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
            .or_insert_with(|| Rc::new(StringCache::from_template(template).unwrap()))
            .clone();

        for segment in entry.segments.iter() {
            match segment {
                Segment::Attr(idx) => {
                    let attr = &template.dynamic_attrs[*idx];
                    match attr.value {
                        AttributeValue::Text(value) => write!(buf, " {}=\"{}\"", attr.name, value)?,
                        AttributeValue::Bool(value) => write!(buf, " {}={}", attr.name, value)?,
                        _ => {}
                    };
                }
                Segment::Node(idx) => match &template.dynamic_nodes[*idx] {
                    DynamicNode::Component(node) => {
                        if self.skip_components {
                            write!(buf, "<{}><{}/>", node.name, node.name)?;
                        } else {
                            let id = node.scope.get().unwrap();
                            let scope = dom.get_scope(id).unwrap();
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
                            write!(buf, "<!--#-->")?;
                        }

                        write!(
                            buf,
                            "{}",
                            askama_escape::escape(text.value, askama_escape::Html)
                        )?;

                        if self.pre_render {
                            write!(buf, "<!--#-->")?;
                        }
                    }
                    DynamicNode::Fragment(nodes) => {
                        for child in *nodes {
                            self.render_template(buf, dom, child)?;
                        }
                    }

                    DynamicNode::Placeholder(_el) => {
                        if self.pre_render {
                            write!(buf, "<pre></pre>")?;
                        }
                    }
                },

                Segment::PreRendered(contents) => write!(buf, "{contents}")?,
            }
        }

        Ok(())
    }
}

#[test]
fn to_string_works() {
    use dioxus::prelude::*;

    fn app(cx: Scope) -> Element {
        let dynamic = 123;
        let dyn2 = "</diiiiiiiiv>"; // this should be escaped

        render! {
            div { class: "asdasdasd", class: "asdasdasd", id: "id-{dynamic}",
                "Hello world 1 -->" "{dynamic}" "<-- Hello world 2"
                div { "nest 1" }
                div {}
                div { "nest 2" }
                "{dyn2}"
                (0..5).map(|i| rsx! { div { "finalize {i}" } })
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    let mut renderer = Renderer::new();
    let out = renderer.render(&dom);

    for item in renderer.template_cache.iter() {
        if item.1.segments.len() > 5 {
            assert_eq!(
                item.1.segments,
                vec![
                    PreRendered("<div class=\"asdasdasd\" class=\"asdasdasd\"".into(),),
                    Attr(0,),
                    PreRendered(">Hello world 1 --&gt;".into(),),
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

    assert_eq!(out, "<div class=\"asdasdasd\" class=\"asdasdasd\" id=\"id-123\">Hello world 1 --&gt;123&lt;-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div>&lt;/diiiiiiiiv&gt;<div>finalize 0</div><div>finalize 1</div><div>finalize 2</div><div>finalize 3</div><div>finalize 4</div></div>");
}
