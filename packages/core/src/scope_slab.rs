use std::ops::{Index, IndexMut};

use bumpslab::{BumpSlab, Slot};
use slab::{Slab, VacantEntry};

use crate::{ScopeId, ScopeState};

/// A thin wrapper around a BumpSlab that uses ids to index into the slab.
pub(crate) struct ScopeSlab {
    slab: BumpSlab<ScopeState>,
    // a slab of slots of stable pointers to the ScopeState in the bump slab
    entries: Slab<Slot<'static, ScopeState>>,
}

impl Drop for ScopeSlab {
    fn drop(&mut self) {
        // Bump slab doesn't drop its contents, so we need to do it manually
        for slot in self.entries.drain() {
            self.slab.remove(slot);
        }
    }
}

impl Default for ScopeSlab {
    fn default() -> Self {
        Self {
            slab: BumpSlab::new(),
            entries: Slab::new(),
        }
    }
}

impl ScopeSlab {
    pub(crate) fn get(&self, id: ScopeId) -> Option<&ScopeState> {
        self.entries.get(id.0).map(|slot| unsafe { &*slot.ptr() })
    }

    pub(crate) fn get_mut(&mut self, id: ScopeId) -> Option<&mut ScopeState> {
        self.entries
            .get(id.0)
            .map(|slot| unsafe { &mut *slot.ptr_mut() })
    }

    pub(crate) fn vacant_entry(&mut self) -> ScopeSlabEntry {
        let entry = self.entries.vacant_entry();
        ScopeSlabEntry {
            slab: &mut self.slab,
            entry,
        }
    }

    pub(crate) fn remove(&mut self, id: ScopeId) {
        self.slab.remove(self.entries.remove(id.0));
    }

    pub(crate) fn contains(&self, id: ScopeId) -> bool {
        self.entries.contains(id.0)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &ScopeState> {
        self.entries.iter().map(|(_, slot)| unsafe { &*slot.ptr() })
    }
}

pub(crate) struct ScopeSlabEntry<'a> {
    slab: &'a mut BumpSlab<ScopeState>,
    entry: VacantEntry<'a, Slot<'static, ScopeState>>,
}

impl<'a> ScopeSlabEntry<'a> {
    pub(crate) fn key(&self) -> ScopeId {
        ScopeId(self.entry.key())
    }

    pub(crate) fn insert(self, scope: ScopeState) -> &'a ScopeState {
        let slot = self.slab.push(scope);
        // this is safe because the slot is only ever accessed with the lifetime of the borrow of the slab
        let slot = unsafe { std::mem::transmute(slot) };
        let entry = self.entry.insert(slot);
        unsafe { &*entry.ptr() }
    }
}

impl Index<ScopeId> for ScopeSlab {
    type Output = ScopeState;
    fn index(&self, id: ScopeId) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl IndexMut<ScopeId> for ScopeSlab {
    fn index_mut(&mut self, id: ScopeId) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}
