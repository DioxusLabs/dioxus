#![doc = include_str!("../docs/head.md")]

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use dioxus_core::{prelude::*, DynamicNode};
use dioxus_core_macro::*;

/// Warn the user if they try to change props on a element that is injected into the head
#[allow(unused)]
fn use_update_warning<T: PartialEq + Clone + 'static>(value: &T, name: &'static str) {
    #[cfg(debug_assertions)]
    {
        let cloned_value = value.clone();
        let initial = use_hook(move || value.clone());

        if initial != cloned_value {
            tracing::warn!("Changing the props of `{name}` is not supported ");
        }
    }
}

fn extract_single_text_node(children: &Element, component: &str) -> Option<String> {
    let vnode = match children {
        Element::Ok(vnode) => vnode,
        Element::Err(err) => {
            tracing::error!("Error while rendering {component}: {err}");
            return None;
        }
    };
    // The title's children must be in one of two forms:
    // 1. rsx! { "static text" }
    // 2. rsx! { "title: {dynamic_text}" }
    match vnode.template {
        // rsx! { "static text" }
        Template {
            roots: &[TemplateNode::Text { text }],
            node_paths: &[],
            attr_paths: &[],
            ..
        } => Some(text.to_string()),
        // rsx! { "title: {dynamic_text}" }
        Template {
            roots: &[TemplateNode::Dynamic { id }],
            node_paths: &[&[0]],
            attr_paths: &[],
            ..
        } => {
            let node = &vnode.dynamic_nodes[id];
            match node {
                DynamicNode::Text(text) => Some(text.value.clone()),
                _ => {
                    tracing::error!("Error while rendering {component}: The children of {component} must be a single text node. It cannot be a component, if statement, loop, or a fragment");
                    None
                }
            }
        }
        _ => {
            tracing::error!(
                "Error while rendering title: The children of title must be a single text node"
            );
            None
        }
    }
}

#[derive(Clone, Props, PartialEq)]
pub struct TitleProps {
    /// The contents of the title tag. The children must be a single text node.
    children: Element,
}

/// Render the title of the page. On web renderers, this will set the [title](crate::elements::title) in the head. On desktop, it will set the window title.
///
/// Unlike most head components, the Title can be modified after the first render. Only the latest update to the title will be reflected if multiple title components are rendered.
///
///
/// The children of the title component must be a single static or formatted string. If there are more children or the children contain components, conditionals, loops, or fragments, the title will not be updated.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn App() -> Element {
///     rsx! {
///         // You can use the Title component to render a title tag into the head of the page or window
///         Title { "My Page" }
///     }
/// }
/// ```
#[component]
pub fn Title(props: TitleProps) -> Element {
    let children = props.children;
    let Some(text) = extract_single_text_node(&children, "Title") else {
        return VNode::empty();
    };

    // Update the title as it changes. NOTE: We don't use use_effect here because we need this to run on the server
    let document = use_hook(document);
    let last_text = use_hook(|| {
        // Set the title initially
        document.set_title(text.clone());
        Rc::new(RefCell::new(text.clone()))
    });

    // If the text changes, update the title
    let mut last_text = last_text.borrow_mut();
    if text != *last_text {
        document.set_title(text.clone());
        *last_text = text;
    }

    VNode::empty()
}

/// Props for the [`Meta`] component
#[derive(Clone, Props, PartialEq)]
pub struct MetaProps {
    pub property: Option<String>,
    pub name: Option<String>,
    pub charset: Option<String>,
    pub http_equiv: Option<String>,
    pub content: Option<String>,
}

impl MetaProps {
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(property) = &self.property {
            attributes.push(("property", property.clone()));
        }
        if let Some(name) = &self.name {
            attributes.push(("name", name.clone()));
        }
        if let Some(charset) = &self.charset {
            attributes.push(("charset", charset.clone()));
        }
        if let Some(http_equiv) = &self.http_equiv {
            attributes.push(("http-equiv", http_equiv.clone()));
        }
        if let Some(content) = &self.content {
            attributes.push(("content", content.clone()));
        }
        attributes
    }
}

/// Render a [`meta`](crate::elements::meta) tag into the head of the page.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn RedirectToDioxusHomepageWithoutJS() -> Element {
///     rsx! {
///         // You can use the meta component to render a meta tag into the head of the page
///         // This meta tag will redirect the user to the dioxuslabs homepage in 10 seconds
///         Meta {
///             http_equiv: "refresh",
///             content: "10;url=https://dioxuslabs.com",
///         }
///     }
/// }
/// ```
///
/// <div class="warning">
///
/// Any updates to the props after the first render will not be reflected in the head.
///
/// </div>
#[component]
pub fn Meta(props: MetaProps) -> Element {
    use_update_warning(&props, "Meta {}");

    use_hook(|| {
        let document = document();
        document.create_meta(props);
    });

    VNode::empty()
}

#[derive(Clone, Props, PartialEq)]
pub struct ScriptProps {
    /// The contents of the script tag. If present, the children must be a single text node.
    pub children: Element,
    /// Scripts are deduplicated by their src attribute
    pub src: Option<String>,
    pub defer: Option<bool>,
    pub crossorigin: Option<String>,
    pub fetchpriority: Option<String>,
    pub integrity: Option<String>,
    pub nomodule: Option<bool>,
    pub nonce: Option<String>,
    pub referrerpolicy: Option<String>,
    pub r#type: Option<String>,
}

impl ScriptProps {
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(defer) = &self.defer {
            attributes.push(("defer", defer.to_string()));
        }
        if let Some(crossorigin) = &self.crossorigin {
            attributes.push(("crossorigin", crossorigin.clone()));
        }
        if let Some(fetchpriority) = &self.fetchpriority {
            attributes.push(("fetchpriority", fetchpriority.clone()));
        }
        if let Some(integrity) = &self.integrity {
            attributes.push(("integrity", integrity.clone()));
        }
        if let Some(nomodule) = &self.nomodule {
            attributes.push(("nomodule", nomodule.to_string()));
        }
        if let Some(nonce) = &self.nonce {
            attributes.push(("nonce", nonce.clone()));
        }
        if let Some(referrerpolicy) = &self.referrerpolicy {
            attributes.push(("referrerpolicy", referrerpolicy.clone()));
        }
        if let Some(r#type) = &self.r#type {
            attributes.push(("type", r#type.clone()));
        }
        if let Some(src) = &self.src {
            attributes.push(("src", src.clone()));
        }
        attributes
    }

    pub fn script_contents(&self) -> Option<String> {
        extract_single_text_node(&self.children, "Script")
    }
}

/// Render a [`script`](crate::elements::script) tag into the head of the page.
///
///
/// If present, the children of the script component must be a single static or formatted string. If there are more children or the children contain components, conditionals, loops, or fragments, the script will not be added.
///
///
/// Any scripts you add will be deduplicated by their `src` attribute (if present).
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn LoadScript() -> Element {
///     rsx! {
///         // You can use the Script component to render a script tag into the head of the page
///         Script {
///             src: asset!("/assets/script.js"),
///         }
///     }
/// }
/// ```
///
/// <div class="warning">
///
/// Any updates to the props after the first render will not be reflected in the head.
///
/// </div>
#[component]
pub fn Script(props: ScriptProps) -> Element {
    use_update_warning(&props, "Script {}");

    use_hook(|| {
        if let Some(src) = &props.src {
            if !should_insert_script(src) {
                return;
            }
        }

        let document = document();
        document.create_script(props);
    });

    VNode::empty()
}

#[derive(Clone, Props, PartialEq)]
pub struct StyleProps {
    /// Styles are deduplicated by their href attribute
    pub href: Option<String>,
    pub media: Option<String>,
    pub nonce: Option<String>,
    pub title: Option<String>,
    /// The contents of the style tag. If present, the children must be a single text node.
    pub children: Element,
}

impl StyleProps {
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(href) = &self.href {
            attributes.push(("href", href.clone()));
        }
        if let Some(media) = &self.media {
            attributes.push(("media", media.clone()));
        }
        if let Some(nonce) = &self.nonce {
            attributes.push(("nonce", nonce.clone()));
        }
        if let Some(title) = &self.title {
            attributes.push(("title", title.clone()));
        }
        attributes
    }

    pub fn style_contents(&self) -> Option<String> {
        extract_single_text_node(&self.children, "Title")
    }
}

/// Render a [`style`](crate::elements::style) tag into the head of the page.
///
/// If present, the children of the style component must be a single static or formatted string. If there are more children or the children contain components, conditionals, loops, or fragments, the style will not be added.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn RedBackground() -> Element {
///     rsx! {
///         // You can use the style component to render a style tag into the head of the page
///         // This style tag will set the background color of the page to red
///         Style {
///             r#"
///                 body {{
///                     background-color: red;
///                 }}
///             "#
///         }
///     }
/// }
/// ```
///
/// <div class="warning">
///
/// Any updates to the props after the first render will not be reflected in the head.
///
/// </div>
#[component]
pub fn Style(props: StyleProps) -> Element {
    use_update_warning(&props, "Style {}");

    use_hook(|| {
        if let Some(href) = &props.href {
            if !should_insert_style(href) {
                return;
            }
        }
        let document = document();
        document.create_style(props);
    });

    VNode::empty()
}

use super::*;

#[derive(Clone, Props, PartialEq)]
pub struct LinkProps {
    pub rel: Option<String>,
    pub media: Option<String>,
    pub title: Option<String>,
    pub disabled: Option<bool>,
    pub r#as: Option<String>,
    pub sizes: Option<String>,
    /// Links are deduplicated by their href attribute
    pub href: Option<String>,
    pub crossorigin: Option<String>,
    pub referrerpolicy: Option<String>,
    pub fetchpriority: Option<String>,
    pub hreflang: Option<String>,
    pub integrity: Option<String>,
    pub r#type: Option<String>,
    pub blocking: Option<String>,
}

impl LinkProps {
    pub fn attributes(&self) -> Vec<(&'static str, String)> {
        let mut attributes = Vec::new();
        if let Some(rel) = &self.rel {
            attributes.push(("rel", rel.clone()));
        }
        if let Some(media) = &self.media {
            attributes.push(("media", media.clone()));
        }
        if let Some(title) = &self.title {
            attributes.push(("title", title.clone()));
        }
        if let Some(disabled) = &self.disabled {
            attributes.push(("disabled", disabled.to_string()));
        }
        if let Some(r#as) = &self.r#as {
            attributes.push(("as", r#as.clone()));
        }
        if let Some(sizes) = &self.sizes {
            attributes.push(("sizes", sizes.clone()));
        }
        if let Some(href) = &self.href {
            attributes.push(("href", href.clone()));
        }
        if let Some(crossorigin) = &self.crossorigin {
            attributes.push(("crossOrigin", crossorigin.clone()));
        }
        if let Some(referrerpolicy) = &self.referrerpolicy {
            attributes.push(("referrerPolicy", referrerpolicy.clone()));
        }
        if let Some(fetchpriority) = &self.fetchpriority {
            attributes.push(("fetchPriority", fetchpriority.clone()));
        }
        if let Some(hreflang) = &self.hreflang {
            attributes.push(("hrefLang", hreflang.clone()));
        }
        if let Some(integrity) = &self.integrity {
            attributes.push(("integrity", integrity.clone()));
        }
        if let Some(r#type) = &self.r#type {
            attributes.push(("type", r#type.clone()));
        }
        if let Some(blocking) = &self.blocking {
            attributes.push(("blocking", blocking.clone()));
        }
        attributes
    }
}

/// Render a [`link`](crate::elements::link) tag into the head of the page.
///
/// > The [Link](https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html) component in dioxus router and this component are completely different.
/// > This component links resources in the head of the page, while the router component creates clickable links in the body of the page.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn RedBackground() -> Element {
///     rsx! {
///         // You can use the meta component to render a meta tag into the head of the page
///         // This meta tag will redirect the user to the dioxuslabs homepage in 10 seconds
///         document::Link {
///             href: asset!("/assets/style.css"),
///             rel: "stylesheet",
///         }
///     }
/// }
/// ```
///
/// <div class="warning">
///
/// Any updates to the props after the first render will not be reflected in the head.
///
/// </div>
#[doc(alias = "<link>")]
#[component]
pub fn Link(props: LinkProps) -> Element {
    use_update_warning(&props, "Link {}");

    use_hook(|| {
        if let Some(href) = &props.href {
            if !should_insert_link(href) {
                return;
            }
        }
        let document = document();
        document.create_link(props);
    });

    VNode::empty()
}

/// Render a `<link>` element with a `rel="stylesheet"` attribute by default.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn App() -> Element {
///     rsx! {
///         Stylesheet {
///             href: "https://example.com/styles.css",
///         }
///     }
/// }
/// ```
#[doc(alias = "<link>")]
#[component]
pub fn Stylesheet(props: LinkProps) -> Element {
    Link(LinkProps {
        rel: Some("stylesheet".to_string()),
        ..props
    })
}

fn get_or_insert_root_context<T: Default + Clone + 'static>() -> T {
    match ScopeId::ROOT.has_context::<T>() {
        Some(context) => context,
        None => {
            let context = T::default();
            ScopeId::ROOT.provide_context(context.clone());
            context
        }
    }
}

#[derive(Default, Clone)]
struct LinkContext(DeduplicationContext);

fn should_insert_link(href: &str) -> bool {
    get_or_insert_root_context::<LinkContext>()
        .0
        .should_insert(href)
}

#[derive(Default, Clone)]
struct ScriptContext(DeduplicationContext);

fn should_insert_script(src: &str) -> bool {
    get_or_insert_root_context::<ScriptContext>()
        .0
        .should_insert(src)
}

#[derive(Default, Clone)]
struct StyleContext(DeduplicationContext);

fn should_insert_style(href: &str) -> bool {
    get_or_insert_root_context::<StyleContext>()
        .0
        .should_insert(href)
}

#[derive(Default, Clone)]
struct DeduplicationContext(Rc<RefCell<HashSet<String>>>);

impl DeduplicationContext {
    fn should_insert(&self, href: &str) -> bool {
        let mut set = self.0.borrow_mut();
        let present = set.contains(href);
        if !present {
            set.insert(href.to_string());
            true
        } else {
            false
        }
    }
}
