//! Store-backed implementations of the generic projector traits.

use crate::scope::SelectorScope;
use crate::store::Store;
use dioxus_signals::project::{
    PathKey, ProjectAwait, ProjectCompose, ProjectLens, ProjectPath, ProjectReact,
};
use dioxus_signals::Readable;

impl<Lens> ProjectLens for SelectorScope<Lens>
where
    Lens: Readable,
    Lens::Target: 'static,
{
    type Lens = Lens;

    type Rebind<U: ?Sized + 'static, L>
        = SelectorScope<L>
    where
        L: Readable<Target = U, Storage = Lens::Storage> + 'static;

    fn project_lens(&self) -> &Lens {
        self.writer()
    }
}

impl<Lens, U, L> ProjectCompose<U, L> for SelectorScope<Lens>
where
    Lens: Readable,
    Lens::Target: 'static,
    U: ?Sized + 'static,
    L: Readable<Target = U, Storage = Lens::Storage> + 'static,
{
    fn project_compose_inner(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L> {
        self.map_writer(map)
    }
}

impl<Lens> ProjectPath for SelectorScope<Lens>
where
    Lens: Readable,
    Lens::Target: 'static,
{
    fn project_key(self, key: PathKey) -> Self {
        self.child_unmapped(key)
    }
}

impl<Lens> ProjectReact for SelectorScope<Lens>
where
    Lens: Readable,
    Lens::Target: 'static,
{
    fn project_track_shallow(&self) {
        SelectorScope::track_shallow(self);
    }

    fn project_track(&self) {
        SelectorScope::track(self);
    }

    fn project_mark_dirty(&self) {
        SelectorScope::mark_dirty(self);
    }

    fn project_mark_dirty_shallow(&self) {
        SelectorScope::mark_dirty_shallow(self);
    }

    fn project_mark_dirty_at_and_after_index(&self, index: usize) {
        SelectorScope::mark_dirty_at_and_after_index(self, index);
    }
}

impl<T, Lens> ProjectLens for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + 'static,
{
    type Lens = Lens;

    type Rebind<U: ?Sized + 'static, L>
        = Store<U, L>
    where
        L: Readable<Target = U, Storage = Lens::Storage> + 'static;

    fn project_lens(&self) -> &Lens {
        self.selector().project_lens()
    }
}

impl<T, Lens, U, L> ProjectCompose<U, L> for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + 'static,
    U: ?Sized + 'static,
    L: Readable<Target = U, Storage = Lens::Storage> + 'static,
{
    fn project_compose_inner(self, map: impl FnOnce(Self::Lens) -> L) -> Self::Rebind<U, L> {
        self.into_selector().project_compose(map).into()
    }
}

impl<T, Lens> ProjectPath for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + 'static,
{
    fn project_key(self, key: PathKey) -> Self {
        self.into_selector().project_key(key).into()
    }
}

impl<T, Lens> ProjectReact for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + 'static,
{
    fn project_track_shallow(&self) {
        self.selector().project_track_shallow();
    }

    fn project_track(&self) {
        self.selector().project_track();
    }

    fn project_mark_dirty(&self) {
        self.selector().project_mark_dirty();
    }

    fn project_mark_dirty_shallow(&self) {
        self.selector().project_mark_dirty_shallow();
    }

    fn project_mark_dirty_at_and_after_index(&self, index: usize) {
        self.selector().project_mark_dirty_at_and_after_index(index);
    }
}

/// Forward [`ProjectAwait`] from a [`Store`]'s lens. This lets any awaitable
/// lens (e.g. a resource lens) propagate through the store wrapper.
impl<T, Lens> ProjectAwait for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: ProjectAwait + Clone + 'static,
{
    type Output = Lens::Output;
    type Future = Lens::Future;
    fn project_future(self) -> Self::Future {
        self.lens().clone().project_future()
    }
}
