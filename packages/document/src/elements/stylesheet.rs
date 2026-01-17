use super::*;

/// Render a [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/link) tag into the head of the page with the stylesheet rel.
/// This is equivalent to the [`Link`](Link) component with a slightly more ergonomic API.
///
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// fn RedBackground() -> Element {
///     rsx! {
///         document::Stylesheet {
///             href: asset!("/assets/style.css")
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
pub fn Stylesheet(props: LinkProps) -> Element {
    super::Link(LinkProps {
        rel: Some("stylesheet".into()),
        r#type: Some("text/css".into()),
        ..props
    })
}
