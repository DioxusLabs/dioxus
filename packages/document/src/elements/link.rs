use super::*;
use crate::document;
use dioxus_html as dioxus_elements;

#[non_exhaustive]
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
    #[props(extends = link, extends = GlobalAttributes)]
    pub additional_attributes: Vec<Attribute>,
}

impl LinkProps {
    /// Get all the attributes for the link tag
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
        let document = document();
        let mut insert_link = document.create_head_component();
        if let Some(href) = &props.href {
            if !should_insert_link(href) {
                insert_link = false;
            }
        }

        if !insert_link {
            return;
        }

        document.create_link(props);
    });

    VNode::empty()
}

#[derive(Default, Clone)]
struct LinkContext(DeduplicationContext);

fn should_insert_link(href: &str) -> bool {
    get_or_insert_root_context::<LinkContext>()
        .0
        .should_insert(href)
}
