use std::hash::Hash;

use crate::ScopeId;

#[derive(Debug, Clone, Eq)]
pub struct DirtyScope {
    pub height: u32,
    pub id: ScopeId,
}

impl PartialOrd for DirtyScope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DirtyScope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height).then(self.id.cmp(&other.id))
    }
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
