//! Dioxus Server-Side-Rendering
//!
//! This crate demonstrates how to implement a custom renderer for Dioxus VNodes via the `TextRenderer` renderer.
//! The `TextRenderer` consumes a Dioxus Virtual DOM, progresses its event queue, and renders the VNodes to a String.
//!
//! While `VNode` supports "to_string" directly, it renders child components as the RSX! macro tokens. For custom components,
//! an external renderer is needed to progress the component lifecycles. The `TextRenderer` shows how to use the Virtual DOM
//! API to progress these lifecycle events to generate a fully-mounted Virtual DOM instance which can be renderer in the
//! `render` method.
//!
//! ```ignore
//! fn main() {
//!     let renderer = TextRenderer::<()>::new(|_| html! {<div> "Hello world" </div>});
//!     let output = renderer.render();
//!     assert_eq!(output, "<div>Hello World</div>");
//! }
//! ```
//!
//! The `TextRenderer` is particularly useful when needing to cache a Virtual DOM in between requests
//!

use dioxus_core::prelude::{VNode, FC};

/// The `TextRenderer` provides a way of rendering a Dioxus Virtual DOM to a String.
///
///
///
pub struct TextRenderer<T> {
    _root_type: std::marker::PhantomData<T>,
}

impl<T> TextRenderer<T> {
    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new(root: FC<T>) -> Self {
        Self {
            _root_type: std::marker::PhantomData {},
        }
    }

    /// Create a new text renderer from an existing Virtual DOM.
    /// This will progress the existing VDom's events to completion.
    pub fn from_vdom() -> Self {
        todo!()
    }

    /// Pass new args to the root function
    pub fn update(&mut self, new_val: T) {
        todo!()
    }

    /// Modify the root function in place, forcing a re-render regardless if the props changed
    pub fn update_mut(&mut self, modifier: impl Fn(&mut T)) {
        todo!()
    }

    /// Immediately render a DomTree to string
    pub fn to_text(root: VNode) -> String {
        todo!()
    }

    /// Render the virtual DOM to a string
    pub fn render(&self) -> String {
        let mut buffer = String::new();

        // iterate through the internal patch queue of virtual dom, and apply them to the buffer
        /*
         */
        todo!()
    }

    /// Render VDom to an existing buffer
    /// TODO @Jon, support non-string buffers to actually make this useful
    /// Currently, this only supports overwriting an existing buffer, instead of just
    pub fn render_mut(&self, buf: &mut String) {
        todo!()
    }
}
