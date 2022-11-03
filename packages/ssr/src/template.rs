use dioxus_core::{prelude::*, AttributeValue};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

/// A virtualdom renderer that caches the templates it has seen for faster rendering
#[derive(Default)]
pub struct SsrRender {
    template_cache: RefCell<HashMap<Template<'static>, Rc<StringCache>>>,
}

struct StringCache {
    segments: Vec<Segment>,
}

#[derive(Default)]
struct StringChain {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
enum Segment {
    Attr(usize),
    Node(usize),
    PreRendered(String),
}

impl std::fmt::Write for StringChain {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        match self.segments.last_mut() {
            Some(Segment::PreRendered(s2)) => s2.push_str(s),
            _ => self.segments.push(Segment::PreRendered(s.to_string())),
        }

        Ok(())
    }
}

impl StringCache {
    fn from_template(template: &VNode) -> Result<Self, std::fmt::Error> {
        let mut chain = StringChain::default();

        let mut cur_path = vec![];

        for (root_idx, root) in template.template.roots.iter().enumerate() {
            Self::recurse(root, &mut cur_path, root_idx, &mut chain)?;
        }

        Ok(Self {
            segments: chain.segments,
        })
    }

    fn recurse(
        root: &TemplateNode,
        cur_path: &mut Vec<usize>,
        root_idx: usize,
        chain: &mut StringChain,
    ) -> Result<(), std::fmt::Error> {
        match root {
            TemplateNode::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                cur_path.push(root_idx);
                write!(chain, "<{}", tag)?;
                for attr in *attrs {
                    match attr {
                        TemplateAttribute::Static { name, value, .. } => {
                            write!(chain, " {}=\"{}\"", name, value)?;
                        }
                        TemplateAttribute::Dynamic(index) => {
                            chain.segments.push(Segment::Attr(*index))
                        }
                    }
                }
                if children.len() == 0 {
                    write!(chain, "/>")?;
                } else {
                    write!(chain, ">")?;
                    for child in *children {
                        Self::recurse(child, cur_path, root_idx, chain)?;
                    }
                    write!(chain, "</{}>", tag)?;
                }
                cur_path.pop();
            }
            TemplateNode::Text(text) => write!(chain, "{}", text)?,
            TemplateNode::Dynamic(idx) | TemplateNode::DynamicText(idx) => {
                chain.segments.push(Segment::Node(*idx))
            }
        }

        Ok(())
    }
}

impl SsrRender {
    pub fn render_vdom(&mut self, dom: &VirtualDom) -> String {
        let scope = dom.base_scope();
        let root = scope.root_node();

        let mut out = String::new();
        self.render_template(&mut out, root).unwrap();

        out
    }

    fn render_template(&self, buf: &mut String, template: &VNode) -> std::fmt::Result {
        let entry = self
            .template_cache
            .borrow_mut()
            .entry(template.template)
            .or_insert_with(|| Rc::new(StringCache::from_template(template).unwrap()))
            .clone();

        for segment in entry.segments.iter() {
            match segment {
                Segment::Attr(idx) => {
                    let attr = &template.dynamic_attrs[*idx];
                    match attr.value {
                        AttributeValue::Text(value) => write!(buf, " {}=\"{}\"", attr.name, value)?,
                        _ => {}
                    };
                }
                Segment::Node(idx) => match &template.dynamic_nodes[*idx] {
                    DynamicNode::Text { value, .. } => {
                        // todo: escape the text
                        write!(buf, "{}", value)?
                    }
                    DynamicNode::Fragment { children } => {
                        for child in *children {
                            self.render_template(buf, child)?;
                        }
                        //
                    }
                    DynamicNode::Component { .. } => {
                        //
                    }
                },

                Segment::PreRendered(text) => buf.push_str(&text),
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
        let dyn2 = "</diiiiiiiiv>"; // todo: escape this

        render! {
            div { class: "asdasdasd", class: "asdasdasd", id: "id-{dynamic}",
                "Hello world 1 -->"
                "{dynamic}"
                "<-- Hello world 2"
                div { "nest 1" }
                div {}
                div { "nest 2" }
                "{dyn2}"

                (0..5).map(|i| rsx! { div { "finalize {i}" } })
            }
        }
    }
    let mut dom = VirtualDom::new(app);

    let mut mutations = Vec::new();
    dom.rebuild(&mut mutations);

    let cache = StringCache::from_template(&dom.base_scope().root_node()).unwrap();
    dbg!(cache.segments);

    let mut renderer = SsrRender::default();
    dbg!(renderer.render_vdom(&dom));
}

#[test]
fn children_processes_properly() {
    use dioxus::prelude::*;

    fn app(cx: Scope) -> Element {
        let d = 123;

        render! {
            div {
                ChildWithChildren {
                    p { "{d}" "hii" }
                }
            }
        }
    }

    #[inline_props]
    fn ChildWithChildren<'a>(cx: Scope<'a>, children: Element<'a>) -> Element {
        render! {
             h1 { children }
        }
    }

    let mut dom = VirtualDom::new(app);

    let mut mutations = vec![];
    dom.rebuild(&mut mutations);
    dbg!(mutations);

    let mut mutations = vec![];
    dom.rebuild(&mut mutations);
    dbg!(mutations);
}
