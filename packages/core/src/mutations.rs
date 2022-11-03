use crate::arena::ElementId;

pub struct Renderer<'a> {
    mutations: Vec<Mutation<'a>>,
}

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
    },

    CreateElement {
        name: &'a str,
        namespace: Option<&'a str>,
        id: ElementId,
    },

    CreateText {
        value: &'a str,
    },

    CreatePlaceholder,

    AppendChildren {
        m: usize,
    },
}
