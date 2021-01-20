//! <div align="center">
//!   <h1>🌗🚀 📦 Dioxus</h1>
//!   <p>
//!     <strong>A concurrent, functional, virtual DOM for Rust</strong>
//!   </p>
//! </div>
//! Dioxus: a concurrent, functional, reactive virtual dom for any renderer in Rust.
//!
//!
//! Dioxus is an efficient virtual dom implementation for building interactive user interfaces in Rust.
//! This crate aims to maintain a uniform hook-based, renderer-agnostic UI framework for cross-platform development.
//!
//! ## Components
//! The base unit of Dioxus is the `component`. Components can be easily created from just a function - no traits required:
//! ```
//! use dioxus_core::prelude::*;
//!
//! fn Example(ctx: Context<()>) -> VNode {
//!     html! { <div> "Hello world!" </div> }
//! }
//! ```
//! Components need to take a "Context" parameter which is generic over some properties. This defines how the component can be used
//! and what properties can be used to specify it in the VNode output. All components in Dioxus are hook-based, which might be more
//! complex than other approaches that use traits + lifecycle events. Alternatively, we provide a "lifecycle hook" if you want more
//! granualar control with behavior similar to other UI frameworks.
//!
//! ## Hooks
//! Dioxus uses hooks for state management. Hooks are a form of state persisted between calls of the function component. Instead of
//! using a single struct to store data, hooks use the "use_hook" building block which allows the persistence of data between
//! function component renders.
//!
//! This allows functions to reuse stateful logic between components, simplify large complex components, and adopt more clear context
//! subscription patterns to make components easier to read.
//!
//! ## Supported Renderers
//! Instead of being tightly coupled to a platform, browser, or toolkit, Dioxus implements a VirtualDOM object which
//! can be consumed to draw the UI. The Dioxus VDOM is reactive and easily consumable by 3rd-party renderers via
//! the `Patch` object. See [Implementing a Renderer](docs/8-custom-renderer.md) and the `StringRenderer` classes for information
//! on how to implement your own custom renderer. We provide 1st-class support for these renderers:
//! - dioxus-desktop (via WebView)
//! - dioxus-web (via WebSys)
//! - dioxus-ssr (via StringRenderer)
//! - dioxus-liveview (SSR + StringRenderer)
//!

/// Re-export common types for ease of development use.
/// Essential when working with the html! macro
///
///
///
pub mod prelude {
    use crate::nodes;
    pub use crate::virtual_dom::{Context, VirtualDom, FC};
    pub use nodes::iterables::IterableNodes;
    pub use nodes::*;

    // TODO @Jon, fix this
    // hack the VNode type until VirtualNode is fixed in the macro crate
    pub type VirtualNode = VNode;

    // Re-export from the macro crate
    pub use dioxus_html_macro::html;
}

/// The Dioxus Virtual Dom integrates an event system and virtual nodes to create reactive user interfaces.
///
/// This module includes all life-cycle related mechanics, including the virtual dom, scopes, properties, and lifecycles.
pub mod virtual_dom {
    use super::*;
    use crate::nodes::VNode;
    use generational_arena::Arena;

    /// An integrated virtual node system that progresses events and diffs UI trees.
    /// Differences are converted into patches which a renderer can use to draw the UI.
    pub struct VirtualDom {
        /// All mounted components are arena allocated to make additions, removals, and references easy to work with
        /// A generational arean is used to re-use slots of deleted scopes without having to resize the underlying arena.
        components: Arena<Scope>,

        /// Components generate lifecycle events
        event_queue: Vec<LifecycleEvent>,
    }

    impl VirtualDom {
        /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
        ///
        /// This means that the root component must either consumes its own context, or statics are used to generate the page.
        /// The root component can access things like routing in its context.
        pub fn new(root: FC<()>) -> Self {
            Self::new_with_props(root)
        }

        /// Start a new VirtualDom instance with a dependent props.
        /// Later, the props can be updated by calling "update" with a new set of props, causing a set of re-renders.
        ///
        /// This is useful when a component tree can be driven by external state (IE SSR) but it would be too expensive
        /// to toss out the entire tree.
        pub fn new_with_props<T>(root: FC<T>) -> Self {
            // Set a first lifecycle event to add the component
            let first_event = LifecycleEvent::Add;

            Self {
                components: Arena::new(),
                event_queue: vec![first_event],
            }
        }
    }

    enum LifecycleEvent {
        Add,
    }

    /// Functional Components leverage the type FC to
    pub type FC<T> = fn(&mut Context<T>) -> VNode;

    /// The Scope that wraps a functional component
    /// Scope's hold subscription, context, and hook information, however, it is allocated on the heap.
    pub struct Scope {
        hook_idx: i32,
        hooks: Vec<()>,
    }

    impl Scope {
        fn new<T>() -> Self {
            Self {
                hook_idx: 0,
                hooks: vec![],
            }
        }
    }

    pub struct HookState {}

    /// Components in Dioxus use the "Context" object to interact with their lifecycle.
    /// This lets components schedule updates, integrate hooks, and expose their context via the context api.
    ///
    /// Properties passed down from the parent component are also directly accessible via the exposed "props" field.
    ///
    /// ```ignore
    /// #[derive(Properties)]
    /// struct Props {
    ///     name: String
    /// }
    ///
    /// fn example(ctx: &Context<Props>) -> VNode {
    ///     html! {
    ///         <div> "Hello, {ctx.props.name}" </div>
    ///     }
    /// }
    /// ```
    pub struct Context<'source, T> {
        _props: std::marker::PhantomData<&'source T>,
    }

    pub trait Properties {}

    // Auto derive for pure components
    impl Properties for () {}

    // Set up a derive macro
    // #[derive(Macro)]
}

/// Virtual Node Support
///
///
///
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn self_closing_tag_to_string() {
            let node = VNode::element("br");

            // No </br> since self closing tag
            assert_eq!(&node.to_string(), "<br>");
        }

        #[test]
        fn to_string() {
            let mut node = VNode::Element(VElement::new("div"));
            node.as_velement_mut()
                .unwrap()
                .attrs
                .insert("id".into(), "some-id".into());

            let mut child = VNode::Element(VElement::new("span"));

            let mut text = VNode::Text(VText::new("Hello world"));

            child.as_velement_mut().unwrap().children.push(text);

            node.as_velement_mut().unwrap().children.push(child);

            let expected = r#"<div id="some-id"><span>Hello world</span></div>"#;

            assert_eq!(node.to_string(), expected);
        }
    }
}

/// The diffing algorithm to compare two VNode trees and generate a list of patches to update the VDom.
/// Currently, using an index-based patching algorithm
///
pub mod diff {
    use super::*;
    use crate::nodes::{VNode, VText};
    use std::cmp::min;
    use std::collections::HashMap;
    use std::mem;

    // pub use apply_patches::patch;

    /// A Patch encodes an operation that modifies a real DOM element.
    ///
    /// To update the real DOM that a user sees you'll want to first diff your
    /// old virtual dom and new virtual dom.
    ///
    /// This diff operation will generate `Vec<Patch>` with zero or more patches that, when
    /// applied to your real DOM, will make your real DOM look like your new virtual dom.
    ///
    /// Each Patch has a u32 node index that helps us identify the real DOM node that it applies to.
    ///
    /// Our old virtual dom's nodes are indexed depth first, as shown in this illustration
    /// (0 being the root node, 1 being it's first child, 2 being it's first child's first child).
    ///
    /// ```text
    ///             .─.
    ///            ( 0 )
    ///             `┬'
    ///         ┌────┴──────┐
    ///         │           │
    ///         ▼           ▼
    ///        .─.         .─.
    ///       ( 1 )       ( 4 )
    ///        `┬'         `─'
    ///    ┌────┴───┐       │
    ///    │        │       ├─────┬─────┐
    ///    ▼        ▼       │     │     │
    ///   .─.      .─.      ▼     ▼     ▼
    ///  ( 2 )    ( 3 )    .─.   .─.   .─.
    ///   `─'      `─'    ( 5 ) ( 6 ) ( 7 )
    ///                    `─'   `─'   `─'
    /// ```
    ///
    /// The patching process is tested in a real browser in crates/virtual-dom-rs/tests/diff_patch.rs
    #[derive(Debug, PartialEq)]
    pub enum Patch<'a> {
        /// Append a vector of child nodes to a parent node id.
        AppendChildren(NodeIdx, Vec<&'a VNode>),
        /// For a `node_i32`, remove all children besides the first `len`
        TruncateChildren(NodeIdx, usize),
        /// Replace a node with another node. This typically happens when a node's tag changes.
        /// ex: <div> becomes <span>
        Replace(NodeIdx, &'a VNode),
        /// Add attributes that the new node has that the old node does not
        AddAttributes(NodeIdx, HashMap<&'a str, &'a str>),
        /// Remove attributes that the old node had that the new node doesn't
        RemoveAttributes(NodeIdx, Vec<&'a str>),
        /// Change the text of a Text node.
        ChangeText(NodeIdx, &'a VText),
    }

    type NodeIdx = usize;

    impl<'a> Patch<'a> {
        /// Every Patch is meant to be applied to a specific node within the DOM. Get the
        /// index of the DOM node that this patch should apply to. DOM nodes are indexed
        /// depth first with the root node in the tree having index 0.
        pub fn node_idx(&self) -> usize {
            match self {
                Patch::AppendChildren(node_idx, _) => *node_idx,
                Patch::TruncateChildren(node_idx, _) => *node_idx,
                Patch::Replace(node_idx, _) => *node_idx,
                Patch::AddAttributes(node_idx, _) => *node_idx,
                Patch::RemoveAttributes(node_idx, _) => *node_idx,
                Patch::ChangeText(node_idx, _) => *node_idx,
            }
        }
    }

    /// Given two VNode's generate Patch's that would turn the old virtual node's
    /// real DOM node equivalent into the new VNode's real DOM node equivalent.
    pub fn diff_vnodes<'a>(old: &'a VNode, new: &'a VNode) -> Vec<Patch<'a>> {
        diff_recursive(&old, &new, &mut 0)
    }

    fn diff_recursive<'a, 'b>(
        old: &'a VNode,
        new: &'a VNode,
        cur_node_idx: &'b mut usize,
    ) -> Vec<Patch<'a>> {
        let mut patches = vec![];
        let mut replace = false;

        // Different enum variants, replace!
        // VNodes are of different types, and therefore will cause a re-render.
        // TODO: Handle previously-mounted children so they don't get re-mounted
        if mem::discriminant(old) != mem::discriminant(new) {
            replace = true;
        }

        if let (VNode::Element(old_element), VNode::Element(new_element)) = (old, new) {
            // Replace if there are different element tags
            if old_element.tag != new_element.tag {
                replace = true;
            }

            // Replace if two elements have different keys
            // TODO: More robust key support. This is just an early stopgap to allow you to force replace
            // an element... say if it's event changed. Just change the key name for now.
            // In the future we want keys to be used to create a Patch::ReOrder to re-order siblings
            if old_element.attrs.get("key").is_some()
                && old_element.attrs.get("key") != new_element.attrs.get("key")
            {
                replace = true;
            }
        }

        // Handle replacing of a node
        if replace {
            patches.push(Patch::Replace(*cur_node_idx, &new));
            if let VNode::Element(old_element_node) = old {
                for child in old_element_node.children.iter() {
                    increment_node_idx_for_children(child, cur_node_idx);
                }
            }
            return patches;
        }

        // The following comparison can only contain identical variants, other
        // cases have already been handled above by comparing variant
        // discriminants.
        match (old, new) {
            // We're comparing two text nodes
            (VNode::Text(old_text), VNode::Text(new_text)) => {
                if old_text != new_text {
                    patches.push(Patch::ChangeText(*cur_node_idx, &new_text));
                }
            }

            // We're comparing two element nodes
            (VNode::Element(old_element), VNode::Element(new_element)) => {
                let mut add_attributes: HashMap<&str, &str> = HashMap::new();
                let mut remove_attributes: Vec<&str> = vec![];

                // TODO: -> split out into func
                for (new_attr_name, new_attr_val) in new_element.attrs.iter() {
                    match old_element.attrs.get(new_attr_name) {
                        Some(ref old_attr_val) => {
                            if old_attr_val != &new_attr_val {
                                add_attributes.insert(new_attr_name, new_attr_val);
                            }
                        }
                        None => {
                            add_attributes.insert(new_attr_name, new_attr_val);
                        }
                    };
                }

                // TODO: -> split out into func
                for (old_attr_name, old_attr_val) in old_element.attrs.iter() {
                    if add_attributes.get(&old_attr_name[..]).is_some() {
                        continue;
                    };

                    match new_element.attrs.get(old_attr_name) {
                        Some(ref new_attr_val) => {
                            if new_attr_val != &old_attr_val {
                                remove_attributes.push(old_attr_name);
                            }
                        }
                        None => {
                            remove_attributes.push(old_attr_name);
                        }
                    };
                }

                if add_attributes.len() > 0 {
                    patches.push(Patch::AddAttributes(*cur_node_idx, add_attributes));
                }
                if remove_attributes.len() > 0 {
                    patches.push(Patch::RemoveAttributes(*cur_node_idx, remove_attributes));
                }

                let old_child_count = old_element.children.len();
                let new_child_count = new_element.children.len();

                if new_child_count > old_child_count {
                    let append_patch: Vec<&'a VNode> =
                        new_element.children[old_child_count..].iter().collect();
                    patches.push(Patch::AppendChildren(*cur_node_idx, append_patch))
                }

                if new_child_count < old_child_count {
                    patches.push(Patch::TruncateChildren(*cur_node_idx, new_child_count))
                }

                let min_count = min(old_child_count, new_child_count);
                for index in 0..min_count {
                    *cur_node_idx = *cur_node_idx + 1;
                    let old_child = &old_element.children[index];
                    let new_child = &new_element.children[index];
                    patches.append(&mut diff_recursive(&old_child, &new_child, cur_node_idx))
                }
                if new_child_count < old_child_count {
                    for child in old_element.children[min_count..].iter() {
                        increment_node_idx_for_children(child, cur_node_idx);
                    }
                }
            }
            (VNode::Text(_), VNode::Element(_)) | (VNode::Element(_), VNode::Text(_)) => {
                unreachable!("Unequal variant discriminants should already have been handled");
            }
            _ => todo!("Diffing Not yet implemented for all node types"),
        };

        //    new_root.create_element()
        patches
    }

    fn increment_node_idx_for_children<'a, 'b>(old: &'a VNode, cur_node_idx: &'b mut usize) {
        *cur_node_idx += 1;
        if let VNode::Element(element_node) = old {
            for child in element_node.children.iter() {
                increment_node_idx_for_children(&child, cur_node_idx);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::prelude::*;
        type VirtualNode = VNode;

        /// Test that we generate the right Vec<Patch> for some start and end virtual dom.
        pub struct DiffTestCase<'a> {
            // ex: "Patching root level nodes works"
            pub description: &'static str,
            // ex: html! { <div> </div> }
            pub old: VNode,
            // ex: html! { <strong> </strong> }
            pub new: VNode,
            // ex: vec![Patch::Replace(0, &html! { <strong></strong> })],
            pub expected: Vec<Patch<'a>>,
        }

        impl<'a> DiffTestCase<'a> {
            pub fn test(&self) {
                // ex: vec![Patch::Replace(0, &html! { <strong></strong> })],
                let patches = diff_vnodes(&self.old, &self.new);

                assert_eq!(patches, self.expected, "{}", self.description);
            }
        }
        use super::*;
        use crate::nodes::{VNode, VText};
        use std::collections::HashMap;

        #[test]
        fn replace_node() {
            DiffTestCase {
                description: "Replace the root if the tag changed",
                old: html! { <div> </div> },
                new: html! { <span> </span> },
                expected: vec![Patch::Replace(0, &html! { <span></span> })],
            }
            .test();
            DiffTestCase {
                description: "Replace a child node",
                old: html! { <div> <b></b> </div> },
                new: html! { <div> <strong></strong> </div> },
                expected: vec![Patch::Replace(1, &html! { <strong></strong> })],
            }
            .test();
            DiffTestCase {
                description: "Replace node with a child",
                old: html! { <div> <b>1</b> <b></b> </div> },
                new: html! { <div> <i>1</i> <i></i> </div>},
                expected: vec![
                    Patch::Replace(1, &html! { <i>1</i> }),
                    Patch::Replace(3, &html! { <i></i> }),
                ], //required to check correct index
            }
            .test();
        }

        #[test]
        fn add_children() {
            DiffTestCase {
                description: "Added a new node to the root node",
                old: html! { <div> <b></b> </div> },
                new: html! { <div> <b></b> <span></span> </div> },
                expected: vec![Patch::AppendChildren(0, vec![&html! { <span></span> }])],
            }
            .test();
        }

        #[test]
        fn remove_nodes() {
            DiffTestCase {
                description: "Remove all child nodes at and after child sibling index 1",
                old: html! { <div> <b></b> <span></span> </div> },
                new: html! { <div> </div> },
                expected: vec![Patch::TruncateChildren(0, 0)],
            }
            .test();
            DiffTestCase {
                description: "Remove a child and a grandchild node",
                old: html! {
                <div>
                 <span>
                   <b></b>
                   // This `i` tag will get removed
                   <i></i>
                 </span>
                 // This `strong` tag will get removed
                 <strong></strong>
                </div> },
                new: html! {
                <div>
                 <span>
                  <b></b>
                 </span>
                </div> },
                expected: vec![Patch::TruncateChildren(0, 1), Patch::TruncateChildren(1, 1)],
            }
            .test();
            DiffTestCase {
                description: "Removing child and change next node after parent",
                old: html! { <div> <b> <i></i> <i></i> </b> <b></b> </div> },
                new: html! { <div> <b> <i></i> </b> <i></i> </div>},
                expected: vec![
                    Patch::TruncateChildren(1, 1),
                    Patch::Replace(4, &html! { <i></i> }),
                ], //required to check correct index
            }
            .test();
        }

        #[test]
        fn add_attributes() {
            let mut attributes = HashMap::new();
            attributes.insert("id", "hello");

            DiffTestCase {
                old: html! { <div> </div> },
                new: html! { <div id="hello"> </div> },
                expected: vec![Patch::AddAttributes(0, attributes.clone())],
                description: "Add attributes",
            }
            .test();

            DiffTestCase {
                old: html! { <div id="foobar"> </div> },
                new: html! { <div id="hello"> </div> },
                expected: vec![Patch::AddAttributes(0, attributes)],
                description: "Change attribute",
            }
            .test();
        }

        #[test]
        fn remove_attributes() {
            DiffTestCase {
                old: html! { <div id="hey-there"></div> },
                new: html! { <div> </div> },
                expected: vec![Patch::RemoveAttributes(0, vec!["id"])],
                description: "Add attributes",
            }
            .test();
        }

        #[test]
        fn change_attribute() {
            let mut attributes = HashMap::new();
            attributes.insert("id", "changed");

            DiffTestCase {
                description: "Add attributes",
                old: html! { <div id="hey-there"></div> },
                new: html! { <div id="changed"> </div> },
                expected: vec![Patch::AddAttributes(0, attributes)],
            }
            .test();
        }

        #[test]
        fn replace_text_node() {
            DiffTestCase {
                description: "Replace text node",
                old: html! { Old },
                new: html! { New },
                expected: vec![Patch::ChangeText(0, &VText::new("New"))],
            }
            .test();
        }

        // Initially motivated by having two elements where all that changed was an event listener
        // because right now we don't patch event listeners. So.. until we have a solution
        // for that we can just give them different keys to force a replace.
        #[test]
        fn replace_if_different_keys() {
            DiffTestCase {
                description: "If two nodes have different keys always generate a full replace.",
                old: html! { <div key="1"> </div> },
                new: html! { <div key="2"> </div> },
                expected: vec![Patch::Replace(0, &html! {<div key="2"> </div>})],
            }
            .test()
        }

        //    // TODO: Key support
        //    #[test]
        //    fn reorder_chldren() {
        //        let mut attributes = HashMap::new();
        //        attributes.insert("class", "foo");
        //
        //        let old_children = vec![
        //            // old node 0
        //            html! { <div key="hello", id="same-id", style="",></div> },
        //            // removed
        //            html! { <div key="gets-removed",> { "This node gets removed"} </div>},
        //            // old node 2
        //            html! { <div key="world", class="changed-class",></div>},
        //            // removed
        //            html! { <div key="this-got-removed",> { "This node gets removed"} </div>},
        //        ];
        //
        //        let new_children = vec![
        //            html! { <div key="world", class="foo",></div> },
        //            html! { <div key="new",> </div>},
        //            html! { <div key="hello", id="same-id",></div>},
        //        ];
        //
        //        test(DiffTestCase {
        //            old: html! { <div> { old_children } </div> },
        //            new: html! { <div> { new_children } </div> },
        //            expected: vec![
        //                // TODO: Come up with the patch structure for keyed nodes..
        //                // keying should only work if all children have keys..
        //            ],
        //            description: "Add attributes",
        //        })
        //    }
    }
}

/// TODO @Jon
/// Figure out if validation should be its own crate, or embedded directly into dioxus
/// Should we even be bothered with validation?
///
///
///
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
    /// ```ignore
    /// use dioxus_core::validation::is_self_closing;
    /// assert_eq!(is_self_closing("br"), true);
    /// assert_eq!(is_self_closing("div"), false);
    /// ```
    pub fn is_self_closing(tag: &str) -> bool {
        SELF_CLOSING_TAGS.contains(tag)
        // SELF_CLOSING_TAGS.contains(tag) || is_self_closing_svg_tag(tag)
    }
}
