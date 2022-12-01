use fxhash::FxHashSet;

use crate::{arena::ElementId, ScopeId};

/// A container for all the relevant steps to modify the Real DOM
///
/// This method returns "mutations" - IE the necessary changes to get the RealDOM to match the VirtualDOM. It also
/// includes a list of NodeRefs that need to be applied and effects that need to be triggered after the RealDOM has
/// applied the edits.
///
/// Mutations are the only link between the RealDOM and the VirtualDOM.
#[derive(Debug, Default)]
#[must_use = "not handling edits can lead to visual inconsistencies in UI"]
pub struct Mutations<'a> {
    /// The ID of the subtree that these edits are targetting
    pub subtree: usize,

    /// The list of Scopes that were diffed, created, and removed during the Diff process.
    pub dirty_scopes: FxHashSet<ScopeId>,

    /// Any mutations required to build the templates using [`Mutations`]
    pub template_edits: Vec<Mutation<'a>>,

    /// Any mutations required to patch the renderer to match the layout of the VirtualDom
    pub dom_edits: Vec<Mutation<'a>>,
}

impl<'a> Mutations<'a> {
    /// Rewrites IDs to just be "template", so you can compare the mutations
    ///
    /// Used really only for testing
    pub fn santize(mut self) -> Self {
        self.template_edits
            .iter_mut()
            .chain(self.dom_edits.iter_mut())
            .for_each(|edit| match edit {
                Mutation::LoadTemplate { name, .. } => *name = "template",
                Mutation::SaveTemplate { name, .. } => *name = "template",
                _ => {}
            });

        self
    }

    /// Push a new mutation into the dom_edits list
    pub(crate) fn push(&mut self, mutation: Mutation<'static>) {
        self.dom_edits.push(mutation)
    }
}

/*
each subtree has its own numbering scheme
*/

#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "type")
)]
#[derive(Debug, PartialEq, Eq)]
pub enum Mutation<'a> {
    /// Pop the topmost node from our stack and append them to the node
    /// at the top of the stack.
    AppendChildren {
        /// How many nodes should be popped from the stack.
        /// The node remaining on the stack will be the target for the append.
        m: usize,
    },

    AssignId {
        path: &'static [u8],
        id: ElementId,
    },

    CreateElement {
        name: &'a str,
    },
    CreateElementNamespace {
        name: &'a str,
        namespace: &'a str,
    },
    CreatePlaceholder {
        id: ElementId,
    },
    CreateStaticPlaceholder,
    CreateTextPlaceholder,
    CreateStaticText {
        value: &'a str,
    },
    CreateTextNode {
        value: &'a str,
        id: ElementId,
    },
    HydrateText {
        path: &'static [u8],
        value: &'a str,
        id: ElementId,
    },
    LoadTemplate {
        name: &'static str,
        index: usize,
        id: ElementId,
    },

    // Take the current element and replace it with the element with the given id.
    ReplaceWith {
        id: ElementId,
        m: usize,
    },

    ReplacePlaceholder {
        path: &'static [u8],
        m: usize,
    },

    /// Insert a number of nodes after a given node.
    InsertAfter {
        /// The ID of the node to insert after.
        id: ElementId,

        /// The ids of the nodes to insert after the target node.
        m: usize,
    },

    /// Insert a number of nodes before a given node.
    InsertBefore {
        /// The ID of the node to insert before.
        id: ElementId,

        /// The ids of the nodes to insert before the target node.
        m: usize,
    },

    /// Save the top m nodes as a placeholder
    SaveTemplate {
        /// The name of the template that we're saving
        name: &'static str,

        /// How many nodes are being saved into this template
        m: usize,
    },

    /// Set the value of a node's attribute.
    SetAttribute {
        /// The name of the attribute to set.
        name: &'a str,
        /// The value of the attribute.
        value: &'a str,

        /// The ID of the node to set the attribute of.
        id: ElementId,

        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'a str>,
    },

    /// Set the value of a node's attribute.
    SetStaticAttribute {
        /// The name of the attribute to set.
        name: &'a str,

        /// The value of the attribute.
        value: &'a str,

        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'a str>,
    },

    /// Set the value of a node's attribute.
    SetBoolAttribute {
        /// The name of the attribute to set.
        name: &'a str,

        /// The value of the attribute.
        value: bool,

        /// The ID of the node to set the attribute of.
        id: ElementId,
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
        scope: ScopeId,

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
