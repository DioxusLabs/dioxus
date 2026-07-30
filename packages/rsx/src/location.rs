use std::{cell::Cell, hash::Hash};

/// A template index within one `rsx!` call.
///
/// Debug hot reload registers templates with `file!()`, `line!()`, and `column!()`, but nested
/// templates in the same macro expansion can share those values. `DynIdx` gives each `TemplateBody`
/// in the call a stable discriminator so the hot-reload map can tell those templates apart.
///
/// The value is assigned after parsing, once `CallBody` can walk the full template tree. That late
/// assignment is why the index lives behind interior mutability.
///
/// Equality intentionally ignores the assigned value so parsed/template equality does not depend on
/// hot-reload metadata.
#[derive(Clone, Debug, Default)]
pub struct DynIdx {
    idx: Cell<Option<usize>>,
}

impl PartialEq for DynIdx {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for DynIdx {}

impl DynIdx {
    pub(crate) fn set(&self, idx: usize) {
        self.idx.set(Some(idx));
    }

    pub fn get(&self) -> usize {
        self.idx.get().unwrap_or(usize::MAX)
    }
}

impl Hash for DynIdx {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.get().hash(state);
    }
}
