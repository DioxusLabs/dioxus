use crate::innerlude::ScopeOrder;
use crate::{virtual_dom::VirtualDom, ScopeId};

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
        Self::PLACEHOLDER
    }
}

impl MountId {
    pub(crate) const PLACEHOLDER: Self = Self(usize::MAX);

    pub(crate) fn as_usize(self) -> Option<usize> {
        if self == Self::PLACEHOLDER {
            None
        } else {
            Some(self.0)
        }
    }

    #[allow(unused)]
    pub(crate) fn mounted(self) -> bool {
        self != Self::PLACEHOLDER
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
        if !self.try_reclaim(el) {
            tracing::error!("cannot reclaim {:?}", el);
        }
    }

    pub(crate) fn try_reclaim(&mut self, el: ElementId) -> bool {
        // We never reclaim the unmounted elements or the root element
        if el.0 == 0 || el.0 == usize::MAX {
            return true;
        }

        self.elements.try_remove(el.0).is_some()
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

        self.dirty_scopes.remove(&ScopeOrder::new(height, id));

        // If this scope was a suspense boundary, remove it from the resolved scopes
        self.resolved_scopes.retain(|s| s != &id);
    }
}

impl ElementPath {
    pub(crate) fn is_descendant(&self, small: &[u8]) -> bool {
        small.len() <= self.path.len() && small == &self.path[..small.len()]
    }
}

#[test]
fn is_descendant() {
    let event_path = ElementPath {
        path: &[1, 2, 3, 4, 5],
    };

    assert!(event_path.is_descendant(&[1, 2, 3, 4, 5]));
    assert!(event_path.is_descendant(&[1, 2, 3, 4]));
    assert!(event_path.is_descendant(&[1, 2, 3]));
    assert!(event_path.is_descendant(&[1, 2]));
    assert!(event_path.is_descendant(&[1]));

    assert!(!event_path.is_descendant(&[1, 2, 3, 4, 5, 6]));
    assert!(!event_path.is_descendant(&[2, 3, 4]));
}

impl PartialEq<&[u8]> for ElementPath {
    fn eq(&self, other: &&[u8]) -> bool {
        self.path.eq(*other)
    }
}
