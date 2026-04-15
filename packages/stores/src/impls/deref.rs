//! `DerefMut`-based projector methods.

use std::ops::DerefMut;

use crate::{ProjectMap, Projected};
use dioxus_signals::Readable;

/// Project through a `DerefMut` target without introducing a new path subscription.
pub trait ProjectDeref<U: ?Sized + 'static>:
    ProjectMap<Lens: Readable<Target: DerefMut<Target = U>>>
{
    /// Project through `DerefMut` to the inner target.
    fn deref(self) -> Projected<Self, U>
    where
        <Self::Lens as Readable>::Target: 'static,
    {
        let map: fn(&<Self::Lens as Readable>::Target) -> &U = |t| &**t;
        let map_mut: fn(&mut <Self::Lens as Readable>::Target) -> &mut U = |t| &mut **t;
        self.project_map(map, map_mut)
    }
}

impl<U: ?Sized + 'static, P> ProjectDeref<U> for P where
    P: ProjectMap<Lens: Readable<Target: DerefMut<Target = U>>>
{
}
