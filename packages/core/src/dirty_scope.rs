use crate::ScopeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirtyScope {
    pub height: u32,
    pub id: ScopeId,
}

impl PartialOrd for DirtyScope {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.height.cmp(&other.height))
    }
}

impl Ord for DirtyScope {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.height.cmp(&other.height)
    }
}
