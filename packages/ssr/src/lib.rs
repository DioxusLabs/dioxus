#![doc = include_str!("../README.md")]

use std::fmt::{Display, Formatter, Write};

use dioxus_core::exports::bumpalo;
use dioxus_core::IntoVNode;
use dioxus_core::*;

fn app(_cx: Scope) -> Element {
    None
}

pub struct SsrRenderer {
    vdom: VirtualDom,
    cfg: SsrConfig,
}

impl SsrRenderer {
    pub fn new(cfg: impl FnOnce(SsrConfig) -> SsrConfig) -> Self {
        Self {
            cfg: cfg(SsrConfig::default()),
            vdom: VirtualDom::new(app),
        }
    }

    pub fn render_lazy<'a>(&'a mut self, f: LazyNodes<'a, '_>) -> String {
        let scope = self.vdom.base_scope();
        let factory = NodeFactory::new(scope);

        let root = f.into_vnode(factory);
        format!(
            "{:}",
            TextRenderer {
                cfg: self.cfg.clone(),
                root: &root,
                vdom: Some(&self.vdom),
            }
        )
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn render_lazy<'a>(f: LazyNodes<'a, '_>) -> String {
    let vdom = VirtualDom::new(app);
    let scope: *const ScopeState = vdom.base_scope();

    // Safety
    //
    // The lifetimes bounds on LazyNodes are really complicated - they need to support the nesting restrictions in
    // regular component usage. The <'a> lifetime is used to enforce that all calls of IntoVnode use the same allocator.
    //
    // When LazyNodes are provided, they are FnOnce, but do not come with a allocator selected to borrow from. The <'a>
    // lifetime is therefore longer than the lifetime of the allocator which doesn't exist... yet.
    //
    // Therefore, we cast our local bump allocator to the right lifetime. This is okay because our usage of the bump
    // arena is *definitely* shorter than the <'a> lifetime, and we return *owned* data - not borrowed data.
    let scope = unsafe { &*scope };

    let root = f.into_vnode(NodeFactory::new(scope));

    let vdom = Some(&vdom);

    let ssr_renderer = TextRenderer {
        cfg: SsrConfig::default(),
        root: &root,
        vdom,
    };
    let r = ssr_renderer.to_string();
    drop(ssr_renderer);
    r
}

pub fn render_vdom(dom: &VirtualDom) -> String {
    format!("{:}", TextRenderer::from_vdom(dom, SsrConfig::default()))
}

pub fn pre_render_vdom(dom: &VirtualDom) -> String {
    format!(
        "{:}",
        TextRenderer::from_vdom(dom, SsrConfig::default().pre_render(true))
    )
}

pub fn render_vdom_cfg(dom: &VirtualDom, cfg: impl FnOnce(SsrConfig) -> SsrConfig) -> String {
    format!(
        "{:}",
        TextRenderer::from_vdom(dom, cfg(SsrConfig::default()))
    )
}

pub fn render_vdom_scope(vdom: &VirtualDom, scope: ScopeId) -> Option<String> {
    Some(format!(
        "{:}",
        TextRenderer {
            cfg: SsrConfig::default(),
            root: vdom.get_scope(scope).unwrap().root_node(),
            vdom: Some(vdom),
        }
    ))
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
/// static App: Component = |cx| cx.render(rsx!(div { "hello world" }));
/// let mut vdom = VirtualDom::new(App);
/// vdom.rebuild();
///
/// let renderer = TextRenderer::new(&vdom);
/// let output = format!("{}", renderer);
/// assert_eq!(output, "<div>hello world</div>");
/// ```
pub struct TextRenderer<'a, 'b, 'c> {
    vdom: Option<&'c VirtualDom>,
    root: &'b VNode<'a>,
    cfg: SsrConfig,
}

impl<'a: 'c, 'c> Display for TextRenderer<'a, '_, 'c> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut last_node_was_text = false;
        self.html_render(self.root, f, 0, &mut last_node_was_text)
    }
}

impl<'a> TextRenderer<'a, '_, 'a> {
    pub fn from_vdom(vdom: &'a VirtualDom, cfg: SsrConfig) -> Self {
        Self {
            cfg,
            root: vdom.base_scope().root_node(),
            vdom: Some(vdom),
        }
    }
}

impl<'a: 'c, 'c> TextRenderer<'a, '_, 'c> {
    fn html_render(
        &self,
        node: &VNode,
        f: &mut impl Write,
        il: u16,
        last_node_was_text: &mut bool,
    ) -> std::fmt::Result {
        match &node {
            VNode::Text(text) => {
                if *last_node_was_text {
                    write!(f, "<!--spacer-->")?;
                }

                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }

                *last_node_was_text = true;

                write!(f, "{}", text.text)?
            }
            VNode::Element(el) => {
                *last_node_was_text = false;

                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }

                write!(f, "<{}", el.tag)?;

                let inner_html = render_attributes(el.attributes.iter(), f)?;

                match self.cfg.newline {
                    true => writeln!(f, ">")?,
                    false => write!(f, ">")?,
                }

                if let Some(inner_html) = inner_html {
                    write!(f, "{}", inner_html)?;
                } else {
                    let mut last_node_was_text = false;
                    for child in el.children {
                        self.html_render(child, f, il + 1, &mut last_node_was_text)?;
                    }
                }

                if self.cfg.newline {
                    writeln!(f)?;
                }
                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }

                write!(f, "</{}>", el.tag)?;
                if self.cfg.newline {
                    writeln!(f)?;
                }
            }
            VNode::Fragment(frag) => match frag.children.len() {
                0 => {
                    *last_node_was_text = false;
                    if self.cfg.indent {
                        for _ in 0..il {
                            write!(f, "    ")?;
                        }
                    }
                    write!(f, "<!--placeholder-->")?;
                }
                _ => {
                    for child in frag.children {
                        self.html_render(child, f, il + 1, last_node_was_text)?;
                    }
                }
            },
            VNode::Component(vcomp) => {
                let idx = vcomp.scope.get().unwrap();

                if let (Some(vdom), false) = (self.vdom, self.cfg.skip_components) {
                    let new_node = vdom.get_scope(idx).unwrap().root_node();
                    self.html_render(new_node, f, il + 1, last_node_was_text)?;
                } else {
                }
            }
            VNode::Template(t) => {
                if let Some(vdom) = self.vdom {
                    todo!()
                } else {
                    panic!("Cannot render template without vdom");
                }
            }
            VNode::Placeholder(_) => {
                todo!()
            }
        }
        Ok(())
    }

    // fn render_template_node(
    //     &self,
    //     node: &VTemplate,
    //     f: &mut impl Write,
    //     last_node_was_text: &mut bool,
    //     il: u16,
    // ) -> std::fmt::Result {
    //     match &node.node_type {
    //         TemplateNodeType::Element(el) => {
    //             *last_node_was_text = false;

    //             if self.cfg.indent {
    //                 for _ in 0..il {
    //                     write!(f, "    ")?;
    //                 }
    //             }

    //             write!(f, "<{}", el.tag)?;

    //             let mut inner_html = None;

    //             let mut attr_iter = el.attributes.as_ref().iter().peekable();

    //             while let Some(attr) = attr_iter.next() {
    //                 match attr.attribute.namespace {
    //                     None => {
    //                         if attr.attribute.name == "dangerous_inner_html" {
    //                             inner_html = {
    //                                 let text = match &attr.value {
    //                                     TemplateAttributeValue::Static(val) => {
    //                                         val.allocate(&self.bump).as_text().unwrap()
    //                                     }
    //                                     TemplateAttributeValue::Dynamic(idx) => dynamic_context
    //                                         .resolve_attribute(*idx)
    //                                         .as_text()
    //                                         .unwrap(),
    //                                 };
    //                                 Some(text)
    //                             }
    //                         } else if is_boolean_attribute(attr.attribute.name) {
    //                             match &attr.value {
    //                                 TemplateAttributeValue::Static(val) => {
    //                                     let val = val.allocate(&self.bump);
    //                                     if val.is_truthy() {
    //                                         write!(f, " {}=\"{}\"", attr.attribute.name, val)?
    //                                     }
    //                                 }
    //                                 TemplateAttributeValue::Dynamic(idx) => {
    //                                     let val = dynamic_context.resolve_attribute(*idx);
    //                                     if val.is_truthy() {
    //                                         write!(f, " {}=\"{}\"", attr.attribute.name, val)?
    //                                     }
    //                                 }
    //                             }
    //                         } else {
    //                             match &attr.value {
    //                                 TemplateAttributeValue::Static(val) => {
    //                                     let val = val.allocate(&self.bump);
    //                                     write!(f, " {}=\"{}\"", attr.attribute.name, val)?
    //                                 }
    //                                 TemplateAttributeValue::Dynamic(idx) => {
    //                                     let val = dynamic_context.resolve_attribute(*idx);
    //                                     write!(f, " {}=\"{}\"", attr.attribute.name, val)?
    //                                 }
    //                             }
    //                         }
    //                     }

    //                     Some(ns) => {
    //                         // write the opening tag
    //                         write!(f, " {}=\"", ns)?;
    //                         let mut cur_ns_el = attr;
    //                         loop {
    //                             match &attr.value {
    //                                 TemplateAttributeValue::Static(val) => {
    //                                     let val = val.allocate(&self.bump);
    //                                     write!(f, "{}:{};", cur_ns_el.attribute.name, val)?;
    //                                 }
    //                                 TemplateAttributeValue::Dynamic(idx) => {
    //                                     let val = dynamic_context.resolve_attribute(*idx);
    //                                     write!(f, "{}:{};", cur_ns_el.attribute.name, val)?;
    //                                 }
    //                             }
    //                             match attr_iter.peek() {
    //                                 Some(next_attr)
    //                                     if next_attr.attribute.namespace == Some(ns) =>
    //                                 {
    //                                     cur_ns_el = attr_iter.next().unwrap();
    //                                 }
    //                                 _ => break,
    //                             }
    //                         }
    //                         // write the closing tag
    //                         write!(f, "\"")?;
    //                     }
    //                 }
    //             }

    //             match self.cfg.newline {
    //                 true => writeln!(f, ">")?,
    //                 false => write!(f, ">")?,
    //             }

    //             if let Some(inner_html) = inner_html {
    //                 write!(f, "{}", inner_html)?;
    //             } else {
    //                 let mut last_node_was_text = false;
    //                 for child in el.children.as_ref() {
    //                     self.render_template_node(
    //                         template_nodes,
    //                         &template_nodes.as_ref()[child.0],
    //                         dynamic_context,
    //                         f,
    //                         &mut last_node_was_text,
    //                         il + 1,
    //                     )?;
    //                 }
    //             }

    //             if self.cfg.newline {
    //                 writeln!(f)?;
    //             }
    //             if self.cfg.indent {
    //                 for _ in 0..il {
    //                     write!(f, "    ")?;
    //                 }
    //             }

    //             write!(f, "</{}>", el.tag)?;
    //             if self.cfg.newline {
    //                 writeln!(f)?;
    //             }
    //         }
    //         TemplateNodeType::Text(txt) => {
    //             if *last_node_was_text {
    //                 write!(f, "<!--spacer-->")?;
    //             }

    //             if self.cfg.indent {
    //                 for _ in 0..il {
    //                     write!(f, "    ")?;
    //                 }
    //             }

    //             *last_node_was_text = true;

    //             let text = dynamic_context.resolve_text(txt);

    //             write!(f, "{}", text)?
    //         }
    //         TemplateNodeType::DynamicNode(idx) => {
    //             let node = dynamic_context.resolve_node(*idx);
    //             self.html_render(node, f, il, last_node_was_text)?;
    //         }
    //     }
    //     Ok(())
    // }
}

fn render_attributes<'a, 'b: 'a>(
    attrs: impl Iterator<Item = &'a Attribute<'b>>,
    f: &mut impl Write,
) -> Result<Option<&'b str>, std::fmt::Error> {
    let mut inner_html = None;
    let mut attr_iter = attrs.peekable();

    while let Some(attr) = attr_iter.next() {
        match attr.namespace {
            None => {
                if attr.name == "dangerous_inner_html" {
                    inner_html = Some(attr.value.as_text().unwrap())
                } else {
                    if is_boolean_attribute(attr.name) && !attr.value.is_truthy() {
                        continue;
                    }
                    write!(f, " {}=\"{}\"", attr.name, attr.value)?
                }
            }
            Some(ns) => {
                // write the opening tag
                write!(f, " {}=\"", ns)?;
                let mut cur_ns_el = attr;
                loop {
                    write!(f, "{}:{};", cur_ns_el.name, cur_ns_el.value)?;
                    match attr_iter.peek() {
                        Some(next_attr) if next_attr.namespace == Some(ns) => {
                            cur_ns_el = attr_iter.next().unwrap();
                        }
                        _ => break,
                    }
                }
                // write the closing tag
                write!(f, "\"")?;
            }
        }
    }
    Ok(inner_html)
}

fn is_boolean_attribute(attribute: &'static str) -> bool {
    matches!(
        attribute,
        "allowfullscreen"
            | "allowpaymentrequest"
            | "async"
            | "autofocus"
            | "autoplay"
            | "checked"
            | "controls"
            | "default"
            | "defer"
            | "disabled"
            | "formnovalidate"
            | "hidden"
            | "ismap"
            | "itemscope"
            | "loop"
            | "multiple"
            | "muted"
            | "nomodule"
            | "novalidate"
            | "open"
            | "playsinline"
            | "readonly"
            | "required"
            | "reversed"
            | "selected"
            | "truespeed"
    )
}

#[derive(Clone, Debug, Default)]
pub struct SsrConfig {
    /// currently not supported - control if we indent the HTML output
    indent: bool,

    /// Control if elements are written onto a new line
    newline: bool,

    /// Choose to write ElementIDs into elements so the page can be re-hydrated later on
    pre_render: bool,

    // Currently not implemented
    // Don't proceed onto new components. Instead, put the name of the component.
    // TODO: components don't have names :(
    skip_components: bool,
}

impl SsrConfig {
    pub fn indent(mut self, a: bool) -> Self {
        self.indent = a;
        self
    }
    pub fn newline(mut self, a: bool) -> Self {
        self.newline = a;
        self
    }
    pub fn pre_render(mut self, a: bool) -> Self {
        self.pre_render = a;
        self
    }
    pub fn skip_components(mut self, a: bool) -> Self {
        self.skip_components = a;
        self
    }
}
