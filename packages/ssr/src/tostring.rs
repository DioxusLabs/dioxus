use std::fmt::Display;

use dioxus_core::prelude::*;
use dioxus_core::{nodes::VNode, prelude::ScopeIdx, virtual_dom::VirtualDom};

struct SsrRenderer {
    dom: VirtualDom,
}

impl Display for SsrRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self
            .dom
            .base_scope()
            // .components
            // .get(self.dom.base_scope)
            // .unwrap()
            .frames
            .current_head_node();

        html_render(&self.dom, node, f)
    }
}

// recursively walk the tree
fn html_render(
    dom: &VirtualDom,
    node: &VNode,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match node {
        VNode::Element(el) => {
            write!(f, "<{}", el.tag_name)?;
            for attr in el.attributes {
                write!(f, " {}=\"{}\"", attr.name, attr.value)?;
            }
            write!(f, ">\n")?;
            for child in el.children {
                html_render(dom, child, f)?;
            }
            write!(f, "\n</{}>", el.tag_name)?;
            Ok(())
        }
        VNode::Text(t) => write!(f, "{}", t.text),
        VNode::Suspended => todo!(),
        VNode::Component(vcomp) => {
            let id = vcomp.ass_scope.as_ref().borrow().unwrap();
            let new_node = dom.components.get(id).unwrap().frames.current_head_node();
            html_render(&dom, new_node, f)
        }
    }
}

#[test]
fn test_serialize() {
    let mut dom = VirtualDom::new(|ctx, props| {
        //
        //
        //
        ctx.render(rsx! {
            div {
                title: "About W3Schools"
                {(0..20).map(|f| rsx!{
                    div {
                        title: "About W3Schools"
                        style: "color:blue;text-align:center"
                        class: "About W3Schools"
                        p {
                            title: "About W3Schools"
                            "Hello world!: {f}"
                        }
                    }
                })}
            }
        })
    });

    dom.rebuild();
    let renderer = SsrRenderer { dom };

    use std::fs::File;
    use std::io::prelude::*;

    let mut file = File::create("index.html").unwrap();
    let buf = renderer.to_string();
    // dbg!(buf);
    file.write(buf.as_bytes());
    // dbg!(renderer.to_string());
}
