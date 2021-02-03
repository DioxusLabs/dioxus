//! Virtual Node Support
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers.
//!
//! These VNodes should be *very* cheap and *very* fast to construct - building a full tree should be insanely quick.

use std::marker::PhantomData;

use bumpalo::Bump;
pub use vcomponent::VComponent;
pub use velement::VElement;
pub use velement::{Attribute, Listener, NodeKey};
pub use vnode::VNode;
pub use vtext::VText;

/// Tools for the base unit of the virtual dom - the VNode
/// VNodes are intended to be quickly-allocated, lightweight enum values.
///
/// Components will be generating a lot of these very quickly, so we want to
/// limit the amount of heap allocations / overly large enum sizes.
mod vnode {
    use super::*;

    pub enum VNode<'src> {
        /// An element node (node type `ELEMENT_NODE`).
        Element(VElement<'src>),

        /// A text node (node type `TEXT_NODE`).
        ///
        /// Note: This wraps a `VText` instead of a plain `String` in
        /// order to enable custom methods like `create_text_node()` on the
        /// wrapped type.
        Text(VText<'src>),

        /// A "suspended component"
        /// This is a masqeurade over an underlying future that needs to complete
        /// When the future is completed, the VNode will then trigger a render
        Suspended,

        /// A User-defined componen node (node type COMPONENT_NODE)
        Component(VComponent),
    }

    impl<'src> VNode<'src> {
        /// Create a new virtual element node with a given tag.
        ///
        /// These get patched into the DOM using `document.createElement`
        ///
        /// ```ignore
        /// let div = VNode::element("div");
        /// ```
        pub fn element(tag: &'static str) -> Self {
            VNode::Element(VElement::new(tag))
        }

        /// Construct a new text node with the given text.
        #[inline]
        pub(crate) fn text(text: &'src str) -> VNode<'src> {
            VNode::Text(VText { text })
        }
        // /// Create a new virtual text node with the given text.
        // ///
        // /// These get patched into the DOM using `document.createTextNode`
        // ///
        // /// ```ignore
        // /// let div = VNode::text("div");
        // /// ```
        // pub fn text<S>(text: S) -> Self
        // where
        //     S: Into<String>,
        // {
        //     /*
        //     TODO

        //     This is an opportunity to be extremely efficient when allocating/creating strings
        //     To assemble a formatted string, we can, using the macro, borrow all the contents without allocating.

        //     String contents are therefore bump allocated automatically

        //     html!{
        //         <>"Hello {world}"</>
        //     }

        //     Should be

        //     ```
        //     let mut root = VNode::text(["Hello", world]);

        //     ```

        //     */
        //     VNode::Text(VText::new(text.into()))
        // }

        // /// Return a [`VElement`] reference, if this is an [`Element`] variant.
        // ///
        // /// [`VElement`]: struct.VElement.html
        // /// [`Element`]: enum.VNode.html#variant.Element
        // pub fn as_velement_ref(&self) -> Option<&VElement> {
        //     match self {
        //         VNode::Element(ref element_node) => Some(element_node),
        //         _ => None,
        //     }
        // }

        // /// Return a mutable [`VElement`] reference, if this is an [`Element`] variant.
        // ///
        // /// [`VElement`]: struct.VElement.html
        // /// [`Element`]: enum.VNode.html#variant.Element
        // pub fn as_velement_mut(&mut self) -> Option<&mut VElement> {
        //     match self {
        //         VNode::Element(ref mut element_node) => Some(element_node),
        //         _ => None,
        //     }
        // }

        // /// Return a [`VText`] reference, if this is an [`Text`] variant.
        // ///
        // /// [`VText`]: struct.VText.html
        // /// [`Text`]: enum.VNode.html#variant.Text
        // pub fn as_vtext_ref(&self) -> Option<&VText> {
        //     match self {
        //         VNode::Text(ref text_node) => Some(text_node),
        //         _ => None,
        //     }
        // }

        // /// Return a mutable [`VText`] reference, if this is an [`Text`] variant.
        // ///
        // /// [`VText`]: struct.VText.html
        // /// [`Text`]: enum.VNode.html#variant.Text
        // pub fn as_vtext_mut(&mut self) -> Option<&mut VText> {
        //     match self {
        //         VNode::Text(ref mut text_node) => Some(text_node),
        //         _ => None,
        //     }
        // }

        // /// Used by html-macro to insert space before text that is inside of a block that came after
        // /// an open tag.
        // ///
        // /// html! { <div> {world}</div> }
        // ///
        // /// So that we end up with <div> world</div> when we're finished parsing.
        // pub fn insert_space_before_text(&mut self) {
        //     match self {
        //         VNode::Text(text_node) => {
        //             text_node.text = " ".to_string() + &text_node.text;
        //         }
        //         _ => {}
        //     }
        // }

        // /// Used by html-macro to insert space after braced text if we know that the next block is
        // /// another block or a closing tag.
        // ///
        // /// html! { <div>{Hello} {world}</div> } -> <div>Hello world</div>
        // /// html! { <div>{Hello} </div> } -> <div>Hello </div>
        // ///
        // /// So that we end up with <div>Hello world</div> when we're finished parsing.
        // pub fn insert_space_after_text(&mut self) {
        //     match self {
        //         VNode::Text(text_node) => {
        //             text_node.text += " ";
        //         }
        //         _ => {}
        //     }
        // }
    }

    // -----------------------------------------------
    //  Convert from DOM elements to the primary enum
    // -----------------------------------------------
    // impl From<VText> for VNode {
    //     fn from(other: VText) -> Self {
    //         VNode::Text(other)
    //     }
    // }

    // impl From<VElement> for VNode {
    //     fn from(other: VElement) -> Self {
    //         VNode::Element(other)
    //     }
    // }
}

mod velement {
    use super::*;
    use std::collections::HashMap;

    pub struct VElement<'a> {
        /// The HTML tag, such as "div"
        pub tag: &'a str,

        pub tag_name: &'a str,
        pub attributes: &'a [Attribute<'a>],
        // todo: hook up listeners
        // pub listeners: &'a [Listener<'a>],
        // / HTML attributes such as id, class, style, etc
        // pub attrs: HashMap<String, String>,
        // TODO: @JON Get this to not heap allocate, but rather borrow
        // pub attrs: HashMap<&'static str, &'static str>,

        // TODO @Jon, re-enable "events"
        //
        // /// Events that will get added to your real DOM element via `.addEventListener`
        // pub events: Events,
        // pub events: HashMap<String, ()>,

        // /// The children of this `VNode`. So a <div> <em></em> </div> structure would
        // /// have a parent div and one child, em.
        // pub children: Vec<VNode>,
    }

    impl<'a> VElement<'a> {
        // The tag of a component MUST be known at compile time
        pub fn new(tag: &'a str) -> Self {
            todo!()
            // VElement {
            //     tag,
            //     attrs: HashMap::new(),
            //     events: HashMap::new(),
            //     // events: Events(HashMap::new()),
            //     children: vec![],
            // }
        }
    }

    /// An attribute on a DOM node, such as `id="my-thing"` or
    /// `href="https://example.com"`.
    #[derive(Clone, Debug)]
    pub struct Attribute<'a> {
        pub(crate) name: &'a str,
        pub(crate) value: &'a str,
    }

    impl<'a> Attribute<'a> {
        /// Get this attribute's name, such as `"id"` in `<div id="my-thing" />`.
        #[inline]
        pub fn name(&self) -> &'a str {
            self.name
        }

        /// The attribute value, such as `"my-thing"` in `<div id="my-thing" />`.
        #[inline]
        pub fn value(&self) -> &'a str {
            self.value
        }

        /// Certain attributes are considered "volatile" and can change via user
        /// input that we can't see when diffing against the old virtual DOM. For
        /// these attributes, we want to always re-set the attribute on the physical
        /// DOM node, even if the old and new virtual DOM nodes have the same value.
        #[inline]
        pub(crate) fn is_volatile(&self) -> bool {
            match self.name {
                "value" | "checked" | "selected" => true,
                _ => false,
            }
        }
    }

    /// An event listener.
    pub struct Listener<'a> {
        /// The type of event to listen for.
        pub(crate) event: &'a str,
        /// The callback to invoke when the event happens.
        pub(crate) callback: &'a (dyn Fn()),
    }

    /// The key for keyed children.
    ///
    /// Keys must be unique among siblings.
    ///
    /// If any sibling is keyed, then they all must be keyed.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct NodeKey(pub(crate) u32);

    impl Default for NodeKey {
        fn default() -> NodeKey {
            NodeKey::NONE
        }
    }
    impl NodeKey {
        /// The default, lack of a key.
        pub const NONE: NodeKey = NodeKey(u32::MAX);

        /// Is this key `NodeKey::NONE`?
        #[inline]
        pub fn is_none(&self) -> bool {
            *self == Self::NONE
        }

        /// Is this key not `NodeKey::NONE`?
        #[inline]
        pub fn is_some(&self) -> bool {
            !self.is_none()
        }

        /// Create a new `NodeKey`.
        ///
        /// `key` must not be `u32::MAX`.
        #[inline]
        pub fn new(key: u32) -> Self {
            debug_assert_ne!(key, u32::MAX);
            NodeKey(key)
        }
    }

    // todo
    // use zst enum for element type. Something like ValidElements::div
}

mod vtext {
    #[derive(PartialEq)]
    pub struct VText<'a> {
        pub text: &'a str,
    }

    impl<'a> VText<'a> {
        // / Create an new `VText` instance with the specified text.
        // pub fn new<S>(text: S) -> Self
        // where
        //     S: Into<String>,
        // {
        //     VText { text: text.into() }
        // }
    }
}

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
mod vcomponent {
    use crate::virtual_dom::Properties;
    use std::{any::TypeId, fmt, future::Future};

    use super::VNode;
    #[derive(PartialEq)]
    pub struct VComponent {
        // props_id: TypeId,
    // callerIDs are unsafely coerced to function pointers
    // This is okay because #1, we store the props_id and verify and 2# the html! macro rejects components not made this way
    //
    // Manually constructing the VComponent is not possible from 3rd party crates
    }

    impl VComponent {
        // /// Construct a VComponent directly from a function component
        // /// This should be *very* fast - we store the function pointer and props type ID. It should also be small on the stack
        // pub fn from_fn<P: Properties>(f: FC<P>, props: P) -> Self {
        //     // // Props needs to be static
        //     // let props_id = std::any::TypeId::of::<P>();

        //     // // Cast the caller down

        //     // Self { props_id }
        //     Self {}
        // }
    }
}
