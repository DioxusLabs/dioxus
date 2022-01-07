#![doc = include_str!("../README.md")]

use std::fmt::{Display, Formatter};

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
        let factory = NodeFactory::new(&scope);

        let root = f.into_vnode(factory);
        format!(
            "{:}",
            TextRenderer {
                cfg: self.cfg.clone(),
                root: &root,
                vdom: None
            }
        )
    }
}

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
    // Therefore, we cast our local bump alloactor into right lifetime. This is okay because our usage of the bump arena
    // is *definitely* shorter than the <'a> lifetime, and we return *owned* data - not borrowed data.
    let scope = unsafe { &*scope };

    let root = f.into_vnode(NodeFactory::new(&scope));

    format!(
        "{:}",
        TextRenderer {
            cfg: SsrConfig::default(),
            root: &root,
            vdom: None
        }
    )
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
            vdom: Some(vdom)
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
pub struct TextRenderer<'a, 'b> {
    vdom: Option<&'a VirtualDom>,
    root: &'b VNode<'a>,
    cfg: SsrConfig,
}

impl Display for TextRenderer<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut last_node_was_text = false;
        self.html_render(self.root, f, 0, &mut last_node_was_text)
    }
}

impl<'a> TextRenderer<'a, '_> {
    pub fn from_vdom(vdom: &'a VirtualDom, cfg: SsrConfig) -> Self {
        Self {
            cfg,
            root: vdom.base_scope().root_node(),
            vdom: Some(vdom),
        }
    }

    fn html_render(
        &self,
        node: &VNode,
        f: &mut std::fmt::Formatter,
        il: u16,
        last_node_was_text: &mut bool,
    ) -> std::fmt::Result {
        match &node {
            VNode::Text(text) => {
                if *last_node_was_text && self.cfg.pre_render {
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
            VNode::Placeholder(_anchor) => {
                *last_node_was_text = false;

                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }
                write!(f, "<!-- -->")?;
            }
            VNode::Element(el) => {
                *last_node_was_text = false;

                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }

                write!(f, "<{}", el.tag)?;

                let mut inner_html = None;
                let mut attr_iter = el.attributes.iter().peekable();

                while let Some(attr) = attr_iter.next() {
                    match attr.namespace {
                        None => match attr.name {
                            "dangerous_inner_html" => inner_html = Some(attr.value),
                            _ => write!(f, " {}=\"{}\"", attr.name, attr.value)?,
                        },

                        Some(ns) => {
                            // write the opening tag
                            write!(f, " {}=\"", ns)?;
                            let mut cur_ns_el = attr;
                            'ns_parse: loop {
                                write!(f, "{}:{};", cur_ns_el.name, cur_ns_el.value)?;
                                match attr_iter.peek() {
                                    Some(next_attr) if next_attr.namespace == Some(ns) => {
                                        cur_ns_el = attr_iter.next().unwrap();
                                    }
                                    _ => break 'ns_parse,
                                }
                            }
                            // write the closing tag
                            write!(f, "\"")?;
                        }
                    }
                }

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
            VNode::Fragment(frag) => {
                for child in frag.children {
                    self.html_render(child, f, il + 1, last_node_was_text)?;
                }
            }
            VNode::Component(vcomp) => {
                let idx = vcomp.scope.get().unwrap();

                if let (Some(vdom), false) = (self.vdom, self.cfg.skip_components) {
                    let new_node = vdom.get_scope(idx).unwrap().root_node();
                    self.html_render(new_node, f, il + 1, last_node_was_text)?;
                } else {
                }
            }
        }
        Ok(())
    }
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
