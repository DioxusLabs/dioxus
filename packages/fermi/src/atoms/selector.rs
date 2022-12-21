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

    pub fn get<O: 'a>(&self, f: impl Readable<O>) -> &'a O {
        let o = f.read(self.root);
        o.unwrap()
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
    fn read<'a>(&self, _root: &'a AtomRoot) -> Option<&'a V> {
        todo!()
    }
    fn init(&self) -> V {
        todo!()
    }
    fn unique_id(&self) -> AtomId {
        todo!()
    }
}
