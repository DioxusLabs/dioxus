use super::*;
use crate::document;
use dioxus_html as dioxus_elements;

#[non_exhaustive]
#[derive(Clone, Props, PartialEq)]
pub struct StyleProps {
    /// Styles are deduplicated by their href attribute
    pub href: Option<String>,
    pub media: Option<String>,
    pub nonce: Option<String>,
    pub title: Option<String>,
    /// The contents of the style tag. If present, the children must be a single text node.
    pub children: Element,
    #[props(extends = style, extends = GlobalAttributes)]
    pub additional_attributes: Vec<Attribute>,
}

impl StyleProps {
    /// Get all the attributes for the style tag
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

    pub fn style_contents(&self) -> Result<String, ExtractSingleTextNodeError<'_>> {
        extract_single_text_node(&self.children)
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
///         document::Style {
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
        let document = document();
        let mut insert_style = document.create_head_component();
        if let Some(href) = &props.href {
            if !should_insert_style(href) {
                insert_style = false;
            }
        }
        if !insert_style {
            return;
        }
        let mut attributes = props.attributes();
        match (&props.href, props.style_contents()) {
            // The style has inline contents, render it as a style tag
            (_, Ok(_)) => document.create_style(props),
            // The style has a src, render it as a link tag
            (Some(_), _) => {
                attributes.push(("type", "text/css".into()));
                document.create_link(LinkProps {
                    media: props.media,
                    title: props.title,
                    r#type: Some("text/css".to_string()),
                    additional_attributes: props.additional_attributes,
                    href: props.href,
                    rel: None,
                    disabled: None,
                    r#as: None,
                    sizes: None,
                    crossorigin: None,
                    referrerpolicy: None,
                    fetchpriority: None,
                    hreflang: None,
                    integrity: None,
                    blocking: None,
                });
            }
            // The style has neither contents nor src, log an error
            (None, Err(err)) => err.log("Style"),
        };
    });

    VNode::empty()
}

#[derive(Default, Clone)]
struct StyleContext(DeduplicationContext);

fn should_insert_style(href: &str) -> bool {
    get_or_insert_root_context::<StyleContext>()
        .0
        .should_insert(href)
}
