use super::cache::Segment;
use dioxus_core::{prelude::*, AttributeValue, DynamicNode, VText};
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

use crate::cache::StringCache;

/// A virtualdom renderer that caches the templates it has seen for faster rendering
#[derive(Default)]
pub struct SsrRender {
    template_cache: HashMap<Template<'static>, Rc<StringCache>>,
}

impl SsrRender {
    pub fn render_vdom(&mut self, dom: &VirtualDom) -> String {
        let scope = dom.base_scope();
        let root = scope.root_node();

        let mut out = String::new();
        // self.render_template(&mut out, dom, root).unwrap();

        out
    }

    fn render_template(
        &mut self,
        buf: &mut String,
        dom: &VirtualDom,
        template: &VNode,
    ) -> std::fmt::Result {
        let entry = self
            .template_cache
            .entry(template.template)
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
                    DynamicNode::Component(_) => todo!(),
                    DynamicNode::Text(_) => todo!(),
                    DynamicNode::Fragment(_) => todo!(),
                    DynamicNode::Placeholder(_) => todo!(),
                    // todo!()
                    // DynamicNode::Text(VText { id, value }) => {
                    //     // in SSR, we are concerned that we can't hunt down the right text node since they might get merged
                    //     // if !*inner {
                    //     write!(buf, "<!--#-->")?;
                    //     // }

                    //     // todo: escape the text
                    //     write!(buf, "{}", value)?;

                    //     // if !*inner {
                    //     write!(buf, "<!--/#-->")?;
                    //     // }
                    // }
                    // DynamicNode::Fragment { nodes, .. } => {
                    //     for child in *nodes {
                    //         self.render_template(buf, dom, child)?;
                    //     }
                    // }
                    // DynamicNode::Component { scope, .. } => {
                    //     let id = scope.get().unwrap();
                    //     let scope = dom.get_scope(id).unwrap();
                    //     self.render_template(buf, dom, scope.root_node())?;
                    // }
                    // DynamicNode::Placeholder(_el) => {
                    //     write!(buf, "<!--placeholder-->")?;
                    // }
                },

                Segment::PreRendered(contents) => buf.push_str(contents),
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
    dom.rebuild();

    use Segment::*;

    // assert_eq!(
    //     StringCache::from_template(&dom.base_scope().root_node())
    //         .unwrap()
    //         .segments,
    //     vec![
    //         PreRendered("<div class=\"asdasdasd\" class=\"asdasdasd\"".into(),),
    //         Attr(0,),
    //         PreRendered(">Hello world 1 -->".into(),),
    //         Node(0,),
    //         PreRendered("<-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div>".into(),),
    //         Node(1,),
    //         Node(2,),
    //         PreRendered("</div>".into(),),
    //     ]
    // );

    // assert_eq!(
    //     SsrRender::default().render_vdom(&dom),
    //     "<div class=\"asdasdasd\" class=\"asdasdasd\" id=\"id-123\">Hello world 1 --><!--#-->123<!--/#--><-- Hello world 2<div>nest 1</div><div></div><div>nest 2</div><!--#--></diiiiiiiiv><!--/#--><div><!--#-->finalize 0<!--/#--></div><div><!--#-->finalize 1<!--/#--></div><div><!--#-->finalize 2<!--/#--></div><div><!--#-->finalize 3<!--/#--></div><div><!--#-->finalize 4<!--/#--></div></div>"
    // );
}
