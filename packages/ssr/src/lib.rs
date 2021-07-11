//! Dioxus Server-Side-Rendering
//!
//! This crate demonstrates how to implement a custom renderer for Dioxus VNodes via the `TextRenderer` renderer.
//! The `TextRenderer` consumes a Dioxus Virtual DOM, progresses its event queue, and renders the VNodes to a String.
//!
//! While `VNode` supports "to_string" directly, it renders child components as the RSX! macro tokens. For custom components,
//! an external renderer is needed to progress the component lifecycles. The `TextRenderer` shows how to use the Virtual DOM
//! API to progress these lifecycle events to generate a fully-mounted Virtual DOM instance which can be renderer in the
//! `render` method.

use std::fmt::{Arguments, Display, Formatter};

use dioxus_core::prelude::*;
use dioxus_core::{nodes::VNode, prelude::ScopeIdx, virtual_dom::VirtualDom};
use std::io::{BufWriter, Result, Write};

pub fn render_root(vdom: &VirtualDom) -> String {
    format!("{:}", TextRenderer::new(vdom))
}

/// A configurable text renderer for the Dioxus VirtualDOM.
///
///
/// ## Details
///
/// This uses the `Formatter` infrastructure so you can write into anything that supports `write_fmt`. We can't accept
/// any generic writer, so you need to "Display" the text renderer. This is done through `format!` or `format_args!`
///
///
///
///
pub struct TextRenderer<'a> {
    vdom: &'a VirtualDom,
}

impl<'a> TextRenderer<'a> {
    fn new(vdom: &'a VirtualDom) -> Self {
        Self { vdom }
    }

    fn html_render(&self, node: &VNode, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match node {
            VNode::Element(el) => {
                write!(f, "<{}", el.tag_name)?;
                for attr in el.attributes {
                    write!(f, " {}=\"{}\"", attr.name, attr.value)?;
                }
                write!(f, ">\n")?;
                for child in el.children {
                    self.html_render(child, f)?;
                }
                write!(f, "\n</{}>", el.tag_name)?;
                Ok(())
            }
            VNode::Text(text) => write!(f, "{}", text.text),
            VNode::Suspended { .. } => todo!(),
            VNode::Component(vcomp) => {
                todo!()
                // let id = vcomp.ass_scope.borrow().unwrap();
                // let id = vcomp.ass_scope.as_ref().borrow().unwrap();
                // let new_node = dom
                //     .components
                //     .try_get(id)
                //     .unwrap()
                //     .frames
                //     .current_head_node();
                // html_render(&dom, new_node, f)
            }
            VNode::Fragment(_) => todo!(),
        }
    }
}

impl Display for TextRenderer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let root = self.vdom.base_scope();
        let root_node = root.root();
        self.html_render(root_node, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dioxus_core as dioxus;
    use dioxus_html as dioxus_elements;

    const SIMPLE_APP: FC<()> = |cx| {
        //
        cx.render(rsx!(div {
            "hello world!"
        }))
    };

    const SLIGHTLY_MORE_COMPLEX: FC<()> = |cx| {
        cx.render(rsx! {
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
    };

    #[test]
    fn test_to_string_works() {
        let mut dom = VirtualDom::new(SIMPLE_APP);
        dom.rebuild_in_place().expect("failed to run virtualdom");
        dbg!(render_root(&dom));
    }

    #[test]
    fn test_write_to_file() {
        use std::fs::File;

        let mut file = File::create("index.html").unwrap();

        let mut dom = VirtualDom::new(SIMPLE_APP);
        dom.rebuild_in_place().expect("failed to run virtualdom");

        file.write_fmt(format_args!("{}", TextRenderer::new(&dom)))
            .unwrap();
    }
}
