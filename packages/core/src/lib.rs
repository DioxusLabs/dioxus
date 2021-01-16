//! Dioxus: a concurrent, functional, virtual dom for any renderer in Rust
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!

/// Re-export common types for ease of development use.
/// Essential when working with the html! macro
pub mod prelude {
    pub use crate::component::Context;
    use crate::nodes;
    pub use crate::renderer::TextRenderer;
    pub use crate::types::FC;
    pub use crate::virtual_dom::VirtualDom;
    pub use nodes::iterables::IterableNodes;
    pub use nodes::*;

    // hack "virtualnode"
    pub type VirtualNode = VNode;

    // Re-export from the macro crate
    pub use html_macro::html;
}

/// The Dioxus Virtual Dom integrates an event system and virtual nodes to create reactive user interfaces.
pub mod virtual_dom {
    use super::*;

    /// An integrated virtual node system that progresses events and diffs UI trees.
    /// Differences are converted into patches which a renderer can use to draw the UI.
    pub struct VirtualDom {}

    impl VirtualDom {
        pub fn new(root: types::FC) -> Self {
            Self {}
        }
    }
}

/// Virtual Node Support
pub mod nodes {
    pub use vcomponent::VComponent;
    pub use velement::VElement;
    pub use vnode::VNode;
    pub use vtext::VText;

    /// Tools for the base unit of the virtual dom - the VNode
    /// VNodes are intended to be quickly-allocated, lightweight enum values.
    ///
    /// Components will be generating a lot of these very quickly, so we want to
    /// limit the amount of heap allocations / overly large enum sizes.
    mod vnode {
        use super::*;

        #[derive(PartialEq)]
        pub enum VNode {
            /// An element node (node type `ELEMENT_NODE`).
            Element(VElement),
            /// A text node (node type `TEXT_NODE`).
            ///
            /// Note: This wraps a `VText` instead of a plain `String` in
            /// order to enable custom methods like `create_text_node()` on the
            /// wrapped type.
            Text(VText),

            /// A User-defined componen node (node type COMPONENT_NODE)
            Component(VComponent),
        }

        impl VNode {
            /// Create a new virtual element node with a given tag.
            ///
            /// These get patched into the DOM using `document.createElement`
            ///
            /// ```ignore
            /// let div = VNode::element("div");
            /// ```
            pub fn element<S>(tag: S) -> Self
            where
                S: Into<String>,
            {
                VNode::Element(VElement::new(tag))
            }

            /// Create a new virtual text node with the given text.
            ///
            /// These get patched into the DOM using `document.createTextNode`
            ///
            /// ```ignore
            /// let div = VNode::text("div");
            /// ```
            pub fn text<S>(text: S) -> Self
            where
                S: Into<String>,
            {
                VNode::Text(VText::new(text.into()))
            }

            /// Return a [`VElement`] reference, if this is an [`Element`] variant.
            ///
            /// [`VElement`]: struct.VElement.html
            /// [`Element`]: enum.VNode.html#variant.Element
            pub fn as_velement_ref(&self) -> Option<&VElement> {
                match self {
                    VNode::Element(ref element_node) => Some(element_node),
                    _ => None,
                }
            }

            /// Return a mutable [`VElement`] reference, if this is an [`Element`] variant.
            ///
            /// [`VElement`]: struct.VElement.html
            /// [`Element`]: enum.VNode.html#variant.Element
            pub fn as_velement_mut(&mut self) -> Option<&mut VElement> {
                match self {
                    VNode::Element(ref mut element_node) => Some(element_node),
                    _ => None,
                }
            }

            /// Return a [`VText`] reference, if this is an [`Text`] variant.
            ///
            /// [`VText`]: struct.VText.html
            /// [`Text`]: enum.VNode.html#variant.Text
            pub fn as_vtext_ref(&self) -> Option<&VText> {
                match self {
                    VNode::Text(ref text_node) => Some(text_node),
                    _ => None,
                }
            }

            /// Return a mutable [`VText`] reference, if this is an [`Text`] variant.
            ///
            /// [`VText`]: struct.VText.html
            /// [`Text`]: enum.VNode.html#variant.Text
            pub fn as_vtext_mut(&mut self) -> Option<&mut VText> {
                match self {
                    VNode::Text(ref mut text_node) => Some(text_node),
                    _ => None,
                }
            }

            /// Used by html-macro to insert space before text that is inside of a block that came after
            /// an open tag.
            ///
            /// html! { <div> {world}</div> }
            ///
            /// So that we end up with <div> world</div> when we're finished parsing.
            pub fn insert_space_before_text(&mut self) {
                match self {
                    VNode::Text(text_node) => {
                        text_node.text = " ".to_string() + &text_node.text;
                    }
                    _ => {}
                }
            }

            /// Used by html-macro to insert space after braced text if we know that the next block is
            /// another block or a closing tag.
            ///
            /// html! { <div>{Hello} {world}</div> } -> <div>Hello world</div>
            /// html! { <div>{Hello} </div> } -> <div>Hello </div>
            ///
            /// So that we end up with <div>Hello world</div> when we're finished parsing.
            pub fn insert_space_after_text(&mut self) {
                match self {
                    VNode::Text(text_node) => {
                        text_node.text += " ";
                    }
                    _ => {}
                }
            }
        }

        // -----------------------------------------------
        //  Convert from DOM elements to the primary enum
        // -----------------------------------------------
        impl From<VText> for VNode {
            fn from(other: VText) -> Self {
                VNode::Text(other)
            }
        }

        impl From<VElement> for VNode {
            fn from(other: VElement) -> Self {
                VNode::Element(other)
            }
        }

        impl From<&str> for VNode {
            fn from(other: &str) -> Self {
                VNode::text(other)
            }
        }

        impl From<String> for VNode {
            fn from(other: String) -> Self {
                VNode::text(other.as_str())
            }
        }

        // -----------------------------------------------
        //  Allow VNodes to be iterated for map-based UI
        // -----------------------------------------------
        impl IntoIterator for VNode {
            type Item = VNode;
            // TODO: Is this possible with an array [VNode] instead of a vec?
            type IntoIter = ::std::vec::IntoIter<VNode>;

            fn into_iter(self) -> Self::IntoIter {
                vec![self].into_iter()
            }
        }

        impl Into<::std::vec::IntoIter<VNode>> for VNode {
            fn into(self) -> ::std::vec::IntoIter<VNode> {
                self.into_iter()
            }
        }

        // -----------------------------------------------
        //  Allow debug/display adherent to the HTML spec
        // -----------------------------------------------
        use std::fmt;
        impl fmt::Debug for VNode {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    VNode::Element(e) => write!(f, "Node::{:?}", e),
                    VNode::Text(t) => write!(f, "Node::{:?}", t),
                    VNode::Component(c) => write!(f, "Node::{:?}", c),
                }
            }
        }

        // Turn a VNode into an HTML string (delegate impl to variants)
        impl fmt::Display for VNode {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    VNode::Element(element) => write!(f, "{}", element),
                    VNode::Text(text) => write!(f, "{}", text),
                    VNode::Component(c) => write!(f, "{}", c),
                }
            }
        }
    }

    mod velement {
        use super::*;
        use std::collections::HashMap;

        #[derive(PartialEq)]
        pub struct VElement {
            /// The HTML tag, such as "div"
            pub tag: String,

            /// HTML attributes such as id, class, style, etc
            pub attrs: HashMap<String, String>,
            // TODO: @JON Get this to not heap allocate, but rather borrow
            // pub attrs: HashMap<&'static str, &'static str>,

            // TODO @Jon, re-enable "events"
            //
            // /// Events that will get added to your real DOM element via `.addEventListener`
            // pub events: Events,
            pub events: HashMap<String, ()>,

            /// The children of this `VNode`. So a <div> <em></em> </div> structure would
            /// have a parent div and one child, em.
            pub children: Vec<VNode>,
        }

        impl VElement {
            pub fn new<S>(tag: S) -> Self
            where
                S: Into<String>,
            {
                VElement {
                    tag: tag.into(),
                    attrs: HashMap::new(),
                    events: HashMap::new(),
                    // events: Events(HashMap::new()),
                    children: vec![],
                }
            }
        }

        // -----------------------------------------------
        //  Allow debug/display adherent to the HTML spec
        // -----------------------------------------------
        use std::fmt;
        impl fmt::Debug for VElement {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f,
                    "Element(<{}>, attrs: {:?}, children: {:?})",
                    self.tag, self.attrs, self.children,
                )
            }
        }

        impl fmt::Display for VElement {
            // Turn a VElement and all of it's children (recursively) into an HTML string
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "<{}", self.tag).unwrap();

                for (attr, value) in self.attrs.iter() {
                    write!(f, r#" {}="{}""#, attr, value)?;
                }

                write!(f, ">")?;

                for child in self.children.iter() {
                    write!(f, "{}", child.to_string())?;
                }

                if !crate::validation::is_self_closing(&self.tag) {
                    write!(f, "</{}>", self.tag)?;
                }

                Ok(())
            }
        }
    }

    mod vtext {
        #[derive(PartialEq)]
        pub struct VText {
            pub text: String,
        }

        impl VText {
            /// Create an new `VText` instance with the specified text.
            pub fn new<S>(text: S) -> Self
            where
                S: Into<String>,
            {
                VText { text: text.into() }
            }
        }

        // -----------------------------------------------
        //  Convert from primitives directly into VText
        // -----------------------------------------------
        impl From<&str> for VText {
            fn from(text: &str) -> Self {
                VText {
                    text: text.to_string(),
                }
            }
        }

        impl From<String> for VText {
            fn from(text: String) -> Self {
                VText { text }
            }
        }

        // -----------------------------------------------
        //  Allow debug/display adherent to the HTML spec
        // -----------------------------------------------
        use std::fmt;
        impl fmt::Debug for VText {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Text({})", self.text)
            }
        }

        // Turn a VText into an HTML string
        impl fmt::Display for VText {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.text)
            }
        }
    }

    /// Virtual Components for custom user-defined components
    /// Only supports the functional syntax
    mod vcomponent {
        #[derive(PartialEq)]
        pub struct VComponent {}

        // -----------------------------------------------
        //  Allow debug/display adherent to the HTML spec
        // -----------------------------------------------
        use std::fmt;
        impl fmt::Debug for VComponent {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                // TODO: @JON Implement how components should be formatted when spit out to html
                // It probably can't be as straightforward as renderinng their VNodes
                // It _could_ be, but we can't really implement that directly
                // Instead, we should drop a vnode labeled with the component id/key

                // write!(
                //     f,
                //     "Element(<{}>, attrs: {:?}, children: {:?})",
                //     self.tag, self.attrs, self.children,
                // )
                Ok(())
            }
        }

        impl fmt::Display for VComponent {
            // Turn a VElement and all of it's children (recursively) into an HTML string
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                // write!(f, "<{}", self.tag).unwrap();

                // for (attr, value) in self.attrs.iter() {
                //     write!(f, r#" {}="{}""#, attr, value)?;
                // }

                // write!(f, ">")?;

                // for child in self.children.iter() {
                //     write!(f, "{}", child.to_string())?;
                // }

                // if !crate::validation::is_self_closing(&self.tag) {
                //     write!(f, "</{}>", self.tag)?;
                // }

                Ok(())
            }
        }
    }

    pub mod iterables {
        use super::*;

        /// Used by the html! macro for all braced child nodes so that we can use any type
        /// that implements Into<IterableNodes>
        ///
        /// html! { <div> { nodes } </div> }
        ///
        /// nodes can be a String .. VNode .. Vec<VNode> ... etc
        pub struct IterableNodes(Vec<VNode>);

        impl IterableNodes {
            /// Retrieve the first node mutably
            pub fn first(&mut self) -> &mut VNode {
                self.0.first_mut().unwrap()
            }

            /// Retrieve the last node mutably
            pub fn last(&mut self) -> &mut VNode {
                self.0.last_mut().unwrap()
            }
        }

        impl IntoIterator for IterableNodes {
            type Item = VNode;
            // TODO: Is this possible with an array [VNode] instead of a vec?
            type IntoIter = ::std::vec::IntoIter<VNode>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl From<VNode> for IterableNodes {
            fn from(other: VNode) -> Self {
                IterableNodes(vec![other])
            }
        }

        impl From<&str> for IterableNodes {
            fn from(other: &str) -> Self {
                IterableNodes(vec![VNode::text(other)])
            }
        }

        impl From<String> for IterableNodes {
            fn from(other: String) -> Self {
                IterableNodes(vec![VNode::text(other.as_str())])
            }
        }

        impl From<Vec<VNode>> for IterableNodes {
            fn from(other: Vec<VNode>) -> Self {
                IterableNodes(other)
            }
        }

        // TODO @Jon
        // Set this up so instead of the view trait, we can just take functions
        // Functions with no context should just be rendered
        // But functions with a context should be treated as regular components

        // impl<V: View> From<Vec<V>> for IterableNodes {
        //     fn from(other: Vec<V>) -> Self {
        //         IterableNodes(other.into_iter().map(|it| it.render()).collect())
        //     }
        // }

        // impl<V: View> From<&Vec<V>> for IterableNodes {
        //     fn from(other: &Vec<V>) -> Self {
        //         IterableNodes(other.iter().map(|it| it.render()).collect())
        //     }
        // }

        // impl<V: View> From<&[V]> for IterableNodes {
        //     fn from(other: &[V]) -> Self {
        //         IterableNodes(other.iter().map(|it| it.render()).collect())
        //     }
        // }
    }
}

pub mod diff {
    pub enum Patch {}
}

/// Example on how to craft a renderer that interacts with the VirtualDom
pub mod renderer {
    use crate::virtual_dom::VirtualDom;

    use super::*;

    /// Renders a full Dioxus app to a String
    ///
    pub struct TextRenderer {}

    impl TextRenderer {
        /// Create a new Text Renderer which renders the VirtualDom to a string
        pub fn new(dom: VirtualDom) -> Self {
            Self {}
        }

        pub fn render(&mut self) -> String {
            todo!()
        }
    }
}

pub mod component {

    /// A wrapper around component contexts that hides component property types
    pub struct AnyContext {}
    pub struct Context<T> {
        _props: std::marker::PhantomData<T>,
    }

    pub trait Properties {}
    impl Properties for () {}

    fn test() {}
}

/// Utility types that wrap internals
pub mod types {
    use super::*;
    use component::{AnyContext, Context};
    use nodes::VNode;

    pub type FC = fn(&mut AnyContext) -> VNode;
}

/// TODO @Jon
/// Figure out if validation should be its own crate, or embedded directly into dioxus
/// Should we even be bothered with validation?
mod validation {
    use once_cell::sync::Lazy;
    use std::collections::HashSet;

    // Used to uniquely identify elements that contain closures so that the DomUpdater can
    // look them up by their unique id.
    // When the DomUpdater sees that the element no longer exists it will drop all of it's
    // Rc'd Closures for those events.
    static SELF_CLOSING_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
        [
            "area", "base", "br", "col", "hr", "img", "input", "link", "meta", "param", "command",
            "keygen", "source",
        ]
        .iter()
        .cloned()
        .collect()
    });

    /// Whether or not this tag is self closing
    ///
    /// ```
    /// use html_validation::is_self_closing;
    ///
    /// assert_eq!(is_self_closing("br"), true);
    ///
    /// assert_eq!(is_self_closing("div"), false);
    /// ```
    pub fn is_self_closing(tag: &str) -> bool {
        SELF_CLOSING_TAGS.contains(tag)
        // SELF_CLOSING_TAGS.contains(tag) || is_self_closing_svg_tag(tag)
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::prelude::*;
    type VirtualNode = VNode;

    /// Test a basic usage of a virtual dom + text renderer combo
    #[test]
    fn simple_integration() {
        let dom = VirtualDom::new(|_| html! { <div>Hello World!</div> });
        let mut renderer = TextRenderer::new(dom);
        let output = renderer.render();
    }
}
