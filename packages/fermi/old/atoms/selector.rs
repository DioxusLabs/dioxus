use crate::{AtomId, AtomRoot, Readable};

pub type Selector<'a, T> = fn(Select<'a>) -> T;

pub struct Select<'a> {
    pub(crate) root: &'a AtomRoot,

    /// Our ID
    selector_id: AtomId,

    /// The ID of any atom we're tracking for updates
    tracked_atoms: Vec<AtomId>,
}

impl<'a> Select<'a> {
    pub fn new(root: &'a AtomRoot, id: AtomId) -> Self {
        Self {
            root,
            tracked_atoms: Vec::new(),
            selector_id: id,
        }
    }

    pub fn get<O>(&self, f: impl Readable<O>) -> &'a O {
        // self.root.register(f, scope);

        todo!()
    }
}
