use super::*;

/// Render a [`link`](crate::elements::link) tag into the head of the page with the stylesheet rel.
/// This is equivalent to the [`Link`](crate::Link) component with a slightly more ergonomic API.
///
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// fn RedBackground() -> Element {
///     rsx! {
///         document::Stylesheet {
///             src: asset!("/assets/style.css")
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
pub fn Stylesheet(props: LinkProps, src: Option<String>) -> Element {
    super::Link(LinkProps {
        href: src.or_else(|| props.href.clone()),
        rel: Some("stylesheet".into()),
        r#type: Some("text/css".into()),
        ..props
    })
}
