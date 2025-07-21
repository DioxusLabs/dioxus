use dioxus_core::{use_hook, VNode};

use crate::document;

use super::*;

#[derive(Clone, Props, PartialEq)]
pub struct TitleProps {
    /// The contents of the title tag. The children must be a single text node.
    children: Element,
}

/// Render the title of the page. On web renderers, this will set the [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/title) in the head. On desktop, it will set the window title.
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
///         document::Title { "My Page" }
///     }
/// }
/// ```
#[component]
#[doc(alias = "<title>")]
pub fn Title(props: TitleProps) -> Element {
    let children = props.children;
    let text = match extract_single_text_node(&children) {
        Ok(text) => text,
        Err(err) => {
            err.log("Title");
            return VNode::empty();
        }
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
