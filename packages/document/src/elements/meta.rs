use super::*;
use crate::document;
use dioxus_html as dioxus_elements;

#[non_exhaustive]
/// Props for the [`Meta`] component
#[derive(Clone, Props, PartialEq)]
pub struct MetaProps {
    pub property: Option<String>,
    pub name: Option<String>,
    pub charset: Option<String>,
    pub http_equiv: Option<String>,
    pub content: Option<String>,
    #[props(extends = meta, extends = GlobalAttributes)]
    pub additional_attributes: Vec<Attribute>,
}

impl MetaProps {
    /// Get all the attributes for the meta tag
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
///         document::Meta {
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
#[doc(alias = "<meta>")]
pub fn Meta(props: MetaProps) -> Element {
    use_update_warning(&props, "Meta {}");

    use_hook(|| {
        let document = document();
        document.create_meta(props);
    });

    VNode::empty()
}
