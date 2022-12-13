use std::hash::Hash;

use crate::ScopeId;

#[derive(Debug, Clone, Eq, PartialOrd, Ord)]
pub struct DirtyScope {
    pub height: u32,
    pub id: ScopeId,
}

impl PartialEq for DirtyScope {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for DirtyScope {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
