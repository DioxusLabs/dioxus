use crate::{AtomId, AtomRoot, Readable, Selector, SelectorId};

pub struct Select<'a> {
    pub(crate) root: &'a AtomRoot,

    /// The ID of any atom we're tracking for updates
    tracked_atoms: Vec<AtomId>,
}

impl<'a> Select<'a> {
    pub fn new(root: &'a AtomRoot) -> Self {
        Self {
            root,
            tracked_atoms: Vec::new(),
        }
    }

    pub fn get<O>(&self, f: impl Readable<O>) -> &'a O {
        // self.root.register(f, scope);

        todo!()
    }

    pub fn select<O>(&self, f: fn(Select<'a>) -> O) -> &O {
        todo!()
    }
}

pub struct SelectorSpecialization;
impl<V, F> Readable<V, SelectorSpecialization> for F
where
    F: Fn(Select) -> V,
{
    fn read(&self, _root: AtomRoot) -> Option<V> {
        todo!()
    }
    fn init(&self) -> V {
        todo!()
    }
    fn unique_id(&self) -> AtomId {
        todo!()
    }
}
