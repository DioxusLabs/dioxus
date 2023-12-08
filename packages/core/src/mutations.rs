use rustc_hash::FxHashSet;

use crate::{arena::ElementId, innerlude::BorrowedAttributeValue, ScopeId, Template};

/// A container for all the relevant steps to modify the Real DOM
///
/// This object provides a bunch of important information for a renderer to use patch the Real Dom with the state of the
/// VirtualDom. This includes the scopes that were modified, the templates that were discovered, and a list of changes
/// in the form of a [`Mutation`].
///
/// These changes are specific to one subtree, so to patch multiple subtrees, you'd need to handle each set separately.
///
/// Templates, however, apply to all subtrees, not just target subtree.
///
/// Mutations are the only link between the RealDOM and the VirtualDOM.
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
#[derive(Debug, Default)]
#[must_use = "not handling edits can lead to visual inconsistencies in UI"]
pub struct Mutations<'a> {
    /// The ID of the subtree that these edits are targetting
    pub subtree: usize,

    /// The list of Scopes that were diffed, created, and removed during the Diff process.
    pub dirty_scopes: FxHashSet<ScopeId>,

    /// Any templates encountered while diffing the DOM.
    ///
    /// These must be loaded into a cache before applying the edits
    pub templates: Vec<Template<'static>>,

    /// Any mutations required to patch the renderer to match the layout of the VirtualDom
    pub edits: Vec<Mutation<'a>>,
}

impl<'a> Mutations<'a> {
    /// Rewrites IDs to just be "template", so you can compare the mutations
    ///
    /// Used really only for testing
    pub fn santize(mut self) -> Self {
        for edit in self.edits.iter_mut() {
            if let Mutation::LoadTemplate { name, .. } = edit {
                *name = "template"
            }
        }

        self
    }

    /// Push a new mutation into the dom_edits list
    pub(crate) fn push(&mut self, mutation: Mutation<'static>) {
        self.edits.push(mutation)
    }
}

/// A `Mutation` represents a single instruction for the renderer to use to modify the UI tree to match the state
/// of the Dioxus VirtualDom.
///
/// These edits can be serialized and sent over the network or through any interface
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
#[derive(Debug, PartialEq)]
pub enum Mutation<'a> {
    /// Add these m children to the target element
    AppendChildren {
        /// The ID of the element being mounted to
        id: ElementId,

        /// The number of nodes on the stack to append to the target element
        m: usize,
    },

    /// Assign the element at the given path the target ElementId.
    ///
    /// The path is in the form of a list of indices based on children. Templates cannot have more than 255 children per
    /// element, hence the use of a single byte.
    ///
    ///
    AssignId {
        /// The path of the child of the topmost node on the stack
        ///
        /// A path of `[]` represents the topmost node. A path of `[0]` represents the first child.
        /// `[0,1,2]` represents 1st child's 2nd child's 3rd child.
        path: &'static [u8],

        /// The ID we're assigning to this element/placeholder.
        ///
        /// This will be used later to modify the element or replace it with another element.
        id: ElementId,
    },

    /// Create a placeholder in the DOM that we will use later.
    ///
    /// Dioxus currently requires the use of placeholders to maintain a re-entrance point for things like list diffing
    CreatePlaceholder {
        /// The ID we're assigning to this element/placeholder.
        ///
        /// This will be used later to modify the element or replace it with another element.
        id: ElementId,
    },

    /// Create a node specifically for text with the given value
    CreateTextNode {
        /// The text content of this text node
        value: &'a str,

        /// The ID we're assigning to this specific text nodes
        ///
        /// This will be used later to modify the element or replace it with another element.
        id: ElementId,
    },

    /// Hydrate an existing text node at the given path with the given text.
    ///
    /// Assign this text node the given ID since we will likely need to modify this text at a later point
    HydrateText {
        /// The path of the child of the topmost node on the stack
        ///
        /// A path of `[]` represents the topmost node. A path of `[0]` represents the first child.
        /// `[0,1,2]` represents 1st child's 2nd child's 3rd child.
        path: &'static [u8],

        /// The value of the textnode that we want to set the placeholder with
        value: &'a str,

        /// The ID we're assigning to this specific text nodes
        ///
        /// This will be used later to modify the element or replace it with another element.
        id: ElementId,
    },

    /// Load and clone an existing node from a template saved under that specific name
    ///
    /// Dioxus guarantees that the renderer will have already been provided the template.
    /// When the template is picked up in the template list, it should be saved under its "name" - here, the name
    LoadTemplate {
        /// The "name" of the template. When paired with `rsx!`, this is autogenerated
        name: &'static str,

        /// Which root are we loading from the template?
        ///
        /// The template is stored as a list of nodes. This index represents the position of that root
        index: usize,

        /// The ID we're assigning to this element being loaded from the template
        ///
        /// This will be used later to move the element around in lists
        id: ElementId,
    },

    /// Replace the target element (given by its ID) with the topmost m nodes on the stack
    ReplaceWith {
        /// The ID of the node we're going to replace with
        id: ElementId,

        /// The number of nodes on the stack to replace the target element with
        m: usize,
    },

    /// Replace an existing element in the template at the given path with the m nodes on the stack
    ReplacePlaceholder {
        /// The path of the child of the topmost node on the stack
        ///
        /// A path of `[]` represents the topmost node. A path of `[0]` represents the first child.
        /// `[0,1,2]` represents 1st child's 2nd child's 3rd child.
        path: &'static [u8],

        /// The number of nodes on the stack to replace the target element with
        m: usize,
    },

    /// Insert a number of nodes after a given node.
    InsertAfter {
        /// The ID of the node to insert after.
        id: ElementId,

        /// The number of nodes on the stack to insert after the target node.
        m: usize,
    },

    /// Insert a number of nodes before a given node.
    InsertBefore {
        /// The ID of the node to insert before.
        id: ElementId,

        /// The number of nodes on the stack to insert before the target node.
        m: usize,
    },

    /// Set the value of a node's attribute.
    SetAttribute {
        /// The name of the attribute to set.
        name: &'a str,

        /// The value of the attribute.
        value: BorrowedAttributeValue<'a>,

        /// The ID of the node to set the attribute of.
        id: ElementId,

        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'a str>,
    },

    /// Set the textcontent of a node.
    SetText {
        /// The textcontent of the node
        value: &'a str,

        /// The ID of the node to set the textcontent of.
        id: ElementId,
    },

    /// Create a new Event Listener.
    NewEventListener {
        /// The name of the event to listen for.
        name: &'a str,

        /// The ID of the node to attach the listener to.
        id: ElementId,
    },

    /// Remove an existing Event Listener.
    RemoveEventListener {
        /// The name of the event to remove.
        name: &'a str,

        /// The ID of the node to remove.
        id: ElementId,
    },

    /// Remove a particular node from the DOM
    Remove {
        /// The ID of the node to remove.
        id: ElementId,
    },

    /// Push the given root node onto our stack.
    PushRoot {
        /// The ID of the root node to push.
        id: ElementId,
    },
}
