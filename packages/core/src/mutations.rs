use crate::{arena::ElementId, ScopeId};

#[derive(Debug)]
pub struct Mutations<'a> {
    pub subtree: usize,
    pub template_mutations: Vec<Mutation<'a>>,
    pub edits: Vec<Mutation<'a>>,
}

impl<'a> Mutations<'a> {
    pub fn new(subtree: usize) -> Self {
        Self {
            subtree,
            edits: Vec::new(),
            template_mutations: Vec::new(),
        }
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
        namespace: Option<&'a str>,
        id: ElementId,
    },

    CreatePlaceholder {
        id: ElementId,
    },

    CreateTextNode {
        value: &'a str,
    },
    HydrateText {
        path: &'static [u8],
        value: &'a str,
        id: ElementId,
    },
    LoadTemplate {
        name: &'static str,
        index: usize,
    },

    // Take the current element and replace it with the element with the given id.
    ReplaceWith {
        id: ElementId,
        m: usize,
    },

    ReplacePlaceholder {
        m: usize,
        path: &'static [u8],
    },

    SaveTemplate {
        name: &'static str,
        m: usize,
    },

    SetAttribute {
        name: &'a str,
        value: &'a str,
        id: ElementId,
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
}
