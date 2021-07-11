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

pub struct SsrConfig {
    // currently not supported - control if we indent the HTML output
    indent: bool,

    // Control if elements are written onto a new line
    newline: bool,

    // Currently not implemented
    // Don't proceed onto new components. Instead, put the name of the component.
    // TODO: components don't have names :(
    skip_components: bool,
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            indent: false,

            newline: false,
            skip_components: false,
        }
    }
}
/// A configurable text renderer for the Dioxus VirtualDOM.
///
///
/// ## Details
///
/// This uses the `Formatter` infrastructure so you can write into anything that supports `write_fmt`. We can't accept
/// any generic writer, so you need to "Display" the text renderer. This is done through `format!` or `format_args!`
///
/// ## Example
/// ```ignore
/// const App: FC<()> = |cx| cx.render(rsx!(div { "hello world" }));
/// let mut vdom = VirtualDom::new(App);
/// vdom.rebuild_in_place();
///
/// let renderer = TextRenderer::new(&vdom);
/// let output = format!("{}", renderer);
/// assert_eq!(output, "<div>hello world</div>");
/// ```
pub struct TextRenderer<'a> {
    vdom: &'a VirtualDom,
    cfg: SsrConfig,
}

impl<'a> TextRenderer<'a> {
    fn new(vdom: &'a VirtualDom) -> Self {
        Self {
            vdom,
            cfg: SsrConfig::default(),
        }
    }

    fn html_render(&self, node: &VNode, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match node {
            VNode::Text(text) => write!(f, "{}", text.text)?,
            VNode::Element(el) => {
                write!(f, "<{}", el.tag_name)?;
                for attr in el.attributes {
                    write!(f, " {}=\"{}\"", attr.name, attr.value)?;
                }
                match self.cfg.newline {
                    true => write!(f, ">\n")?,
                    false => write!(f, ">")?,
                }

                for child in el.children {
                    self.html_render(child, f)?;
                }
                match self.cfg.newline {
                    true => write!(f, "\n</{}>", el.tag_name)?,
                    false => write!(f, "</{}>", el.tag_name)?,
                }
            }
            VNode::Fragment(frag) => {
                for child in frag.children {
                    self.html_render(child, f)?;
                }
            }
            VNode::Component(vcomp) => {
                let idx = vcomp.ass_scope.get().unwrap();

                let new_node = self
                    .vdom
                    .components
                    .try_get(idx)
                    .unwrap()
                    .frames
                    .current_head_node();

                self.html_render(new_node, f)?;
            }
            VNode::Suspended { .. } => todo!(),
        }
        Ok(())
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

        let mut dom = VirtualDom::new(SLIGHTLY_MORE_COMPLEX);
        dom.rebuild_in_place().expect("failed to run virtualdom");

        file.write_fmt(format_args!("{}", TextRenderer::new(&dom)))
            .unwrap();
    }
}
