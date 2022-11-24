use crate::{arena::ElementId, ScopeId};

#[derive(Debug)]
pub struct Mutations<'a> {
    pub subtree: usize,
    pub template_mutations: Vec<Mutation<'a>>,
    pub edits: Vec<Mutation<'a>>,
}

impl<'a> Mutations<'a> {
    pub fn new() -> Self {
        Self {
            subtree: 0,
            edits: Vec::new(),
            template_mutations: Vec::new(),
        }
    }

    /// A useful tool for testing mutations
    ///
    /// Rewrites IDs to just be "template", so you can compare the mutations
    pub fn santize(mut self) -> Self {
        for edit in self
            .template_mutations
            .iter_mut()
            .chain(self.edits.iter_mut())
        {
            match edit {
                Mutation::LoadTemplate { name, .. } => *name = "template",
                Mutation::SaveTemplate { name, .. } => *name = "template",
                _ => {}
            }
        }

        self
    }
}

impl<'a> std::ops::Deref for Mutations<'a> {
    type Target = Vec<Mutation<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.edits
    }
}

impl std::ops::DerefMut for Mutations<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.edits
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
    AppendChildren {
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

    SaveTemplate {
        name: &'static str,
        m: usize,
    },

    SetAttribute {
        name: &'a str,
        value: &'a str,
        id: ElementId,

        // value: &'bump str,
        /// The (optional) namespace of the attribute.
        /// For instance, "style" is in the "style" namespace.
        ns: Option<&'a str>,
    },

    SetStaticAttribute {
        name: &'a str,
        value: &'a str,
        ns: Option<&'a str>,
    },

    SetBoolAttribute {
        name: &'a str,
        value: bool,
        id: ElementId,
    },

    SetInnerText {
        value: &'a str,
    },

    SetText {
        value: &'a str,
        id: ElementId,
    },

    /// Create a new Event Listener.
    NewEventListener {
        /// The name of the event to listen for.
        event_name: &'a str,

        /// The ID of the node to attach the listener to.
        scope: ScopeId,

        /// The ID of the node to attach the listener to.
        id: ElementId,
    },

    /// Remove an existing Event Listener.
    RemoveEventListener {
        /// The ID of the node to remove.
        id: ElementId,

        /// The name of the event to remove.
        event: &'a str,
    },
    Remove {
        id: ElementId,
    },
}
