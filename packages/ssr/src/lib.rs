//!
//!
//!
//!
//! This crate demonstrates how to implement a custom renderer for Dioxus VNodes via the `TextRenderer` renderer.
//! The `TextRenderer` consumes a Dioxus Virtual DOM, progresses its event queue, and renders the VNodes to a String.
//!
//! While `VNode` supports "to_string" directly, it renders child components as the RSX! macro tokens. For custom components,
//! an external renderer is needed to progress the component lifecycles. The `TextRenderer` shows how to use the Virtual DOM
//! API to progress these lifecycle events to generate a fully-mounted Virtual DOM instance which can be renderer in the
//! `render` method.

use std::fmt::{Display, Formatter};

use dioxus_core::exports::bumpalo;
use dioxus_core::exports::bumpalo::Bump;
use dioxus_core::IntoVNode;
use dioxus_core::*;

/// A memory pool for rendering
pub struct SsrRenderer {
    inner: bumpalo::Bump,
    cfg: SsrConfig,
}

impl SsrRenderer {
    pub fn new(cfg: impl FnOnce(SsrConfig) -> SsrConfig) -> Self {
        Self {
            cfg: cfg(SsrConfig::default()),
            inner: bumpalo::Bump::new(),
        }
    }

    pub fn render_lazy<'a>(&'a mut self, f: Option<LazyNodes<'a, '_>>) -> String {
        let bump = &mut self.inner as *mut _;
        let s = self.render_inner(f);
        // reuse the bump's memory
        unsafe { (&mut *bump as &mut bumpalo::Bump).reset() };
        s
    }

    fn render_inner<'a>(&'a self, f: Option<LazyNodes<'a, '_>>) -> String {
        let factory = NodeFactory::new(&self.inner);
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

pub fn render_lazy<'a>(f: Option<LazyNodes<'a, '_>>) -> String {
    let bump = bumpalo::Bump::new();
    let borrowed = &bump;

    // Safety
    //
    // The lifetimes bounds on LazyNodes are really complicated - they need to support the nesting restrictions in
    // regular component usage. The <'a> lifetime is used to enforce that all calls of IntoVnode use the same allocator.
    //
    // When LazyNodes are provided, they are FnOnce, but do not come with a allocator selected to borrow from. The <'a>
    // lifetime is therefore longer than the lifetime of the allocator which doesn't exist yet.
    //
    // Therefore, we cast our local bump alloactor into right lifetime. This is okay because our usage of the bump arena
    // is *definitely* shorter than the <'a> lifetime, and we return owned data - not borrowed data.
    // Therefore, we know that no references are leaking.
    let _b: &'a Bump = unsafe { std::mem::transmute(borrowed) };

    let root = f.into_vnode(NodeFactory::new(_b));

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
/// static App: Component<()> = |cx, props|cx.render(rsx!(div { "hello world" }));
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
        self.html_render(self.root, f, 0)
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

    fn html_render(&self, node: &VNode, f: &mut std::fmt::Formatter, il: u16) -> std::fmt::Result {
        match &node {
            VNode::Text(text) => {
                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }
                write!(f, "{}", text.text)?
            }
            VNode::Placeholder(_anchor) => {
                //
                if self.cfg.indent {
                    for _ in 0..il {
                        write!(f, "    ")?;
                    }
                }
                write!(f, "<!-- -->")?;
            }
            VNode::Element(el) => {
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

                // we write the element's id as a data attribute
                //
                // when the page is loaded, the `querySelectorAll` will be used to collect all the nodes, and then add
                // them interpreter's stack
                if let (true, Some(id)) = (self.cfg.pre_render, node.try_mounted_id()) {
                    write!(f, " dio_el=\"{}\"", id)?;

                    for _listener in el.listeners {
                        // todo: write the listeners
                    }
                }

                match self.cfg.newline {
                    true => writeln!(f, ">")?,
                    false => write!(f, ">")?,
                }

                if let Some(inner_html) = inner_html {
                    write!(f, "{}", inner_html)?;
                } else {
                    for child in el.children {
                        self.html_render(child, f, il + 1)?;
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
                    self.html_render(child, f, il + 1)?;
                }
            }
            VNode::Component(vcomp) => {
                let idx = vcomp.scope.get().unwrap();

                if let (Some(vdom), false) = (self.vdom, self.cfg.skip_components) {
                    let new_node = vdom.get_scope(idx).unwrap().root_node();
                    self.html_render(new_node, f, il + 1)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    use dioxus_core::prelude::*;
    use dioxus_core_macro::*;
    use dioxus_html as dioxus_elements;

    static SIMPLE_APP: Component<()> = |cx| {
        cx.render(rsx!(div {
            "hello world!"
        }))
    };

    static SLIGHTLY_MORE_COMPLEX: Component<()> = |cx| {
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

    static NESTED_APP: Component<()> = |cx| {
        cx.render(rsx!(
            div {
                SIMPLE_APP {}
            }
        ))
    };
    static FRAGMENT_APP: Component<()> = |cx| {
        cx.render(rsx!(
            div { "f1" }
            div { "f2" }
            div { "f3" }
            div { "f4" }
        ))
    };

    #[test]
    fn to_string_works() {
        let mut dom = VirtualDom::new(SIMPLE_APP);
        dom.rebuild();
        dbg!(render_vdom(&dom));
    }

    #[test]
    fn hydration() {
        let mut dom = VirtualDom::new(NESTED_APP);
        dom.rebuild();
        dbg!(render_vdom_cfg(&dom, |c| c.pre_render(true)));
    }

    #[test]
    fn nested() {
        let mut dom = VirtualDom::new(NESTED_APP);
        dom.rebuild();
        dbg!(render_vdom(&dom));
    }

    #[test]
    fn fragment_app() {
        let mut dom = VirtualDom::new(FRAGMENT_APP);
        dom.rebuild();
        dbg!(render_vdom(&dom));
    }

    #[test]
    fn write_to_file() {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create("index.html").unwrap();

        let mut dom = VirtualDom::new(SLIGHTLY_MORE_COMPLEX);
        dom.rebuild();

        file.write_fmt(format_args!(
            "{}",
            TextRenderer::from_vdom(&dom, SsrConfig::default())
        ))
        .unwrap();
    }

    #[test]
    fn styles() {
        static STLYE_APP: Component<()> = |cx| {
            cx.render(rsx! {
                div { color: "blue", font_size: "46px"  }
            })
        };

        let mut dom = VirtualDom::new(STLYE_APP);
        dom.rebuild();
        dbg!(render_vdom(&dom));
    }

    #[test]
    fn lazy() {
        let p1 = SsrRenderer::new(|c| c).render_lazy(rsx! {
            div { "ello"  }
        });

        let p2 = render_lazy(rsx! {
            div {
                "ello"
            }
        });
        assert_eq!(p1, p2);
    }

    #[test]
    fn big_lazy() {
        let s = render_lazy(rsx! {
            div {
                div {
                    div {
                        h1 { "ello world" }
                        h1 { "ello world" }
                        h1 { "ello world" }
                        h1 { "ello world" }
                        h1 { "ello world" }
                    }
                }
            }
        });

        dbg!(s);
    }

    #[test]
    fn inner_html() {
        let s = render_lazy(rsx! {
            div {
                dangerous_inner_html: "<div> ack </div>"
            }
        });

        dbg!(s);
    }
}
