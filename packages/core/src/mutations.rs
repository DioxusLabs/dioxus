use crate::arena::ElementId;

#[derive(Debug)]
pub struct Renderer<'a> {
    pub subtree: usize,
    pub template_mutations: Vec<Mutation<'a>>,
    pub mutations: Vec<Mutation<'a>>,
}

impl<'a> Renderer<'a> {
    pub fn new(subtree: usize) -> Self {
        Self {
            subtree,
            mutations: Vec::new(),
            template_mutations: Vec::new(),
        }
    }
}

impl<'a> std::ops::Deref for Renderer<'a> {
    type Target = Vec<Mutation<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.mutations
    }
}

impl std::ops::DerefMut for Renderer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mutations
    }
}

// impl<'a> Renderer<'a> {
//     pub fn new(subtree: usize) -> Self {
//         Self {
//             mutations: vec![Mutations {
//                 subtree,
//                 mutations: Vec::new(),
//             }],
//         }
//     }
// }
// impl<'a> Renderer<'a> {
//     pub fn push(&mut self, mutation: Mutation<'a>) {
//         self.mutations.last_mut().unwrap().mutations.push(mutation)
//     }

//     pub fn extend(&mut self, mutations: impl IntoIterator<Item = Mutation<'a>>) {
//         self.mutations
//             .last_mut()
//             .unwrap()
//             .mutations
//             .extend(mutations)
//     }

//     pub fn len(&self) -> usize {
//         self.mutations.last().unwrap().mutations.len()
//     }

//     pub fn split_off(&mut self, idx: usize) -> Renderer<'a> {
//         let mut mutations = self.mutations.split_off(idx);
//         let subtree = mutations.pop().unwrap().subtree;
//         Renderer { mutations }
//     }
// }

// #[derive(Debug)]
// pub struct Mutations<'a> {
//     subtree: usize,
//     mutations: Vec<Mutation<'a>>,
// }

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
