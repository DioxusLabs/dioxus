use crate::{innerlude::DirtyScope, virtual_dom::VirtualDom, ScopeId};

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ElementId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ElementId(pub usize);

/// An Element that can be bubbled to's unique identifier.
///
/// `BubbleId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `BubbleId` will be reused for a new component.
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MountId(pub(crate) usize);

impl Default for MountId {
    fn default() -> Self {
        Self(usize::MAX)
    }
}

impl MountId {
    pub(crate) fn as_usize(self) -> Option<usize> {
        if self.0 == usize::MAX {
            None
        } else {
            Some(self.0)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementRef {
    // the pathway of the real element inside the template
    pub(crate) path: ElementPath,

    // The actual element
    pub(crate) mount: MountId,
}

#[derive(Clone, Copy, Debug)]
pub struct ElementPath {
    pub(crate) path: &'static [u8],
}

impl VirtualDom {
    pub(crate) fn next_element(&mut self) -> ElementId {
        ElementId(self.elements.insert(None))
    }

    pub(crate) fn reclaim(&mut self, el: ElementId) {
        self.try_reclaim(el)
            .unwrap_or_else(|| panic!("cannot reclaim {:?}", el));
    }

    pub(crate) fn try_reclaim(&mut self, el: ElementId) -> Option<()> {
        if el.0 == 0 {
            panic!("Cannot reclaim the root element",);
        }

        self.elements.try_remove(el.0).map(|_| ())
    }

    // Drop a scope without dropping its children
    //
    // Note: This will not remove any ids from the arena
    pub(crate) fn drop_scope(&mut self, id: ScopeId) {
        let height = {
            let scope = self.scopes.remove(id.0);
            let context = scope.state();
            context.height
        };

        self.dirty_scopes.remove(&DirtyScope { height, id });
    }
}

impl ElementPath {
    pub(crate) fn is_decendant(&self, small: &&[u8]) -> bool {
        small.len() <= self.path.len() && *small == &self.path[..small.len()]
    }
}

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        self.path.eq(*other)
    }
}
