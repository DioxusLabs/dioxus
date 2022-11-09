use crate::arena::ElementId;

#[derive(Debug)]
pub struct Mutations<'a> {
    pub subtree: usize,
    pub template_mutations: Vec<Mutation<'a>>,
    pub mutations: Vec<Mutation<'a>>,
}

impl<'a> Mutations<'a> {
    pub fn new(subtree: usize) -> Self {
        Self {
            subtree,
            mutations: Vec::new(),
            template_mutations: Vec::new(),
        }
    }
}

impl<'a> std::ops::Deref for Mutations<'a> {
    type Target = Vec<Mutation<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.mutations
    }
}

impl std::ops::DerefMut for Mutations<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mutations
    }
}

/*
each subtree has its own numbering scheme
*/

#[derive(Debug)]
pub enum Mutation<'a> {
    SetAttribute {
        name: &'a str,
        value: &'a str,
        id: ElementId,
    },

    LoadTemplate {
        name: &'static str,
        index: usize,
    },

    SaveTemplate {
        name: &'static str,
        m: usize,
    },

    HydrateText {
        path: &'static [u8],
        value: &'a str,
        id: ElementId,
    },

    SetText {
        value: &'a str,
        id: ElementId,
    },

    ReplacePlaceholder {
        m: usize,
        path: &'static [u8],
    },

    AssignId {
        path: &'static [u8],
        id: ElementId,
    },

    // Take the current element and replace it with the element with the given id.
    Replace {
        id: ElementId,
        m: usize,
    },

    CreateElement {
        name: &'a str,
        namespace: Option<&'a str>,
        id: ElementId,
    },

    CreateText {
        value: &'a str,
    },

    CreatePlaceholder {
        id: ElementId,
    },

    AppendChildren {
        m: usize,
    },
}
