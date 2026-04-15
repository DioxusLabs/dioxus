//! Generic projection primitives.
//!
//! `Project` is the core trait that abstracts over "scope into a child lens".
//! It's implemented by:
//!
//! - [`crate::scope::SelectorScope`] — for stores; records path keys in the
//!   subscription tree so writes only notify interested subscribers.
//! - [`LensOnly`] — for resources (and any other lens-only view); ignores path
//!   keys entirely, paying no subscription/tracking cost.
//!
//! Shape-specific projection methods (`transpose` / `ok` / `err` / `index` /
//! `deref` / …) are defined as default methods on trait bounded by `Project`,
//! so adding a new shape adds code once and it works on both `Store` and
//! resource-backed types automatically.

use std::ops::DerefMut;

use crate::subscriptions::PathKey;
use crate::scope::SelectorScope;
use dioxus_signals::{
    BorrowError, BorrowMutError, MappedMutSignal, Readable, ReadableExt, ReadableRef, Writable,
    WritableExt, WritableRef,
};
use dioxus_core::{ReactiveContext, Subscribers};

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

/// Abstracts over "a thing you can scope into a child". The two implementors
/// are [`SelectorScope`] (tracked) and [`LensOnly`] (untracked).
pub trait Project: Sized + Copy {
    /// The lens this projection reads/writes through.
    type Lens: Readable + Copy;

    /// The type of a projected child. It's also `Project` so projections
    /// compose inductively.
    type Child<U: ?Sized + 'static, F: Copy + 'static, FMut: Copy + 'static>: Project<
            Lens = MappedMutSignal<U, Self::Lens, F, FMut>,
        >
    where
        F: Fn(&<Self::Lens as Readable>::Target) -> &U,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U;

    /// Scope into a keyed child, wrapping the lens with a new map/map_mut.
    /// `SelectorScope` interprets the key as a position in the subscription
    /// tree; `LensOnly` ignores it.
    fn project_child<U, F, FMut>(
        self,
        key: PathKey,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        <Self::Lens as Readable>::Target: 'static,
        F: Fn(&<Self::Lens as Readable>::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U + 'static + Copy;

    /// Map the lens without introducing a path-level child (used for things
    /// like `Deref` / `as_slice` that project to a different view of the same
    /// cell without new subscription granularity).
    fn project_map<U, F, FMut>(
        self,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        <Self::Lens as Readable>::Target: 'static,
        F: Fn(&<Self::Lens as Readable>::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut <Self::Lens as Readable>::Target) -> &mut U + 'static + Copy;

    /// Track this projection's scope shallowly (no-op for LensOnly).
    fn project_track_shallow(&self);

    /// Track this projection recursively (no-op for LensOnly).
    fn project_track(&self);

    /// Mark this scope as dirty at the root path (no-op for LensOnly — the
    /// signal write itself notifies its subscribers).
    fn project_mark_dirty(&self);

    /// Mark this scope shallow-dirty (length/structure changed but not every element).
    fn project_mark_dirty_shallow(&self);

    /// Mark elements at and after `index` dirty (for sequence structures).
    fn project_mark_dirty_at_and_after_index(&self, index: usize);

    /// Mark the child at a hashed key dirty (for map-like structures).
    /// No-op for LensOnly; SelectorScope hashes the key and marks the child path.
    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, key: &K);

    /// Borrow the underlying lens.
    fn project_lens(&self) -> &Self::Lens;

    /// Write to the lens without any tracking/notifying (fine-grained
    /// dirty-marking is done by `project_mark_dirty*`).
    fn project_write_untracked(&self) -> WritableRef<'static, Self::Lens>
    where
        Self::Lens: Writable,
        <Self::Lens as Readable>::Target: 'static;

    /// Read the current value without tracking.
    fn project_peek(&self) -> ReadableRef<'static, Self::Lens>
    where
        <Self::Lens as Readable>::Target: 'static,
    {
        self.project_lens().peek_unchecked()
    }
}

// ---------------------------------------------------------------------------
// SelectorScope impl — tracked
// ---------------------------------------------------------------------------

impl<Lens> Project for SelectorScope<Lens>
where
    Lens: Readable + Copy,
    Lens::Target: 'static,
{
    type Lens = Lens;

    type Child<U: ?Sized + 'static, F: Copy + 'static, FMut: Copy + 'static> = SelectorScope<MappedMutSignal<U, Lens, F, FMut>>
    where
        F: Fn(&Lens::Target) -> &U,
        FMut: Fn(&mut Lens::Target) -> &mut U;

    fn project_child<U, F, FMut>(
        self,
        key: PathKey,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        F: Fn(&Lens::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut Lens::Target) -> &mut U + 'static + Copy,
    {
        SelectorScope::child(self, key, map, map_mut)
    }

    fn project_map<U, F, FMut>(
        self,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        F: Fn(&Lens::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut Lens::Target) -> &mut U + 'static + Copy,
    {
        SelectorScope::map(self, map, map_mut)
    }

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

    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, key: &K) {
        // SelectorScope is Copy when Lens: Copy — ok to consume a shallow clone.
        let child = (*self).hash_child_unmapped(key);
        SelectorScope::mark_dirty(&child);
    }

    fn project_lens(&self) -> &Lens {
        self.writer()
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Lens>
    where
        Lens: Writable,
        Lens::Target: 'static,
    {
        self.write_untracked()
    }
}

// ---------------------------------------------------------------------------
// Store — delegates to its internal SelectorScope. This is what lets every
// shape trait (ProjectOption, ProjectResult, …) work on `Store` directly,
// with no per-method shims.
// ---------------------------------------------------------------------------

use crate::store::Store;

impl<T, Lens> Project for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T> + Copy + 'static,
{
    type Lens = Lens;

    type Child<U: ?Sized + 'static, F: Copy + 'static, FMut: Copy + 'static> = Store<U, MappedMutSignal<U, Lens, F, FMut>>
    where
        F: Fn(&T) -> &U,
        FMut: Fn(&mut T) -> &mut U;

    fn project_child<U, F, FMut>(
        self,
        key: PathKey,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        F: Fn(&T) -> &U + 'static + Copy,
        FMut: Fn(&mut T) -> &mut U + 'static + Copy,
    {
        self.into_selector().project_child(key, map, map_mut).into()
    }

    fn project_map<U, F, FMut>(
        self,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        F: Fn(&T) -> &U + 'static + Copy,
        FMut: Fn(&mut T) -> &mut U + 'static + Copy,
    {
        self.into_selector().project_map(map, map_mut).into()
    }

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

    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, key: &K) {
        self.selector().project_mark_hash_child_dirty(key);
    }

    fn project_lens(&self) -> &Lens {
        self.selector().project_lens()
    }

    fn project_write_untracked(&self) -> WritableRef<'static, Lens>
    where
        Lens: Writable,
        Lens::Target: 'static,
    {
        self.selector().project_write_untracked()
    }
}

// ---------------------------------------------------------------------------
// LensOnly — untracked lens projection, used by resources
// ---------------------------------------------------------------------------

/// A lens wrapped for projection without a subscription tree. Used by
/// resources: the underlying signal already notifies subscribers on change, so
/// per-path tracking is pure overhead for this case.
pub struct LensOnly<L> {
    lens: L,
}

impl<L: Copy> Copy for LensOnly<L> {}
impl<L: Clone> Clone for LensOnly<L> {
    fn clone(&self) -> Self {
        LensOnly { lens: self.lens.clone() }
    }
}

impl<L> LensOnly<L> {
    /// Wrap a lens.
    pub fn new(lens: L) -> Self {
        LensOnly { lens }
    }

    /// Unwrap into the inner lens.
    pub fn into_inner(self) -> L {
        self.lens
    }
}

impl<L: Readable> Readable for LensOnly<L> {
    type Target = L::Target;
    type Storage = L::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.lens.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError>
    where
        Self::Target: 'static,
    {
        self.lens.try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        Self::Target: 'static,
    {
        self.lens.subscribers()
    }
}

impl<L: Writable> Writable for LensOnly<L> {
    type WriteMetadata = L::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.lens.try_write_unchecked()
    }
}

impl<L> std::fmt::Display for LensOnly<L>
where
    L: Readable + 'static,
    L::Target: std::fmt::Display + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| std::fmt::Display::fmt(v, f))
    }
}

impl<L> std::fmt::Debug for LensOnly<L>
where
    L: Readable + 'static,
    L::Target: std::fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with(|v| std::fmt::Debug::fmt(v, f))
    }
}

impl<L> PartialEq for LensOnly<L>
where
    L: Readable + PartialEq + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.lens == other.lens
    }
}

impl<L> std::ops::Deref for LensOnly<L>
where
    L: Readable + 'static,
    L::Target: Clone + 'static + Sized,
{
    type Target = dyn Fn() -> L::Target;
    fn deref(&self) -> &Self::Target {
        // Safety: same pattern as Store's Deref impl.
        unsafe { ReadableExt::deref_impl(self) }
    }
}


impl<L> Project for LensOnly<L>
where
    L: Readable + Copy,
    L::Target: 'static,
{
    type Lens = L;

    type Child<U: ?Sized + 'static, F: Copy + 'static, FMut: Copy + 'static> = LensOnly<MappedMutSignal<U, L, F, FMut>>
    where
        F: Fn(&L::Target) -> &U,
        FMut: Fn(&mut L::Target) -> &mut U;

    fn project_child<U, F, FMut>(
        self,
        _key: PathKey,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        F: Fn(&L::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut L::Target) -> &mut U + 'static + Copy,
    {
        LensOnly {
            lens: MappedMutSignal::new(self.lens, map, map_mut),
        }
    }

    fn project_map<U, F, FMut>(
        self,
        map: F,
        map_mut: FMut,
    ) -> Self::Child<U, F, FMut>
    where
        U: ?Sized + 'static,
        F: Fn(&L::Target) -> &U + 'static + Copy,
        FMut: Fn(&mut L::Target) -> &mut U + 'static + Copy,
    {
        LensOnly {
            lens: MappedMutSignal::new(self.lens, map, map_mut),
        }
    }

    fn project_track_shallow(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.lens.subscribers());
        }
    }
    fn project_track(&self) {
        if let Some(rc) = ReactiveContext::current() {
            rc.subscribe(self.lens.subscribers());
        }
    }
    fn project_mark_dirty(&self) {
        // LensOnly has no path tree — a signal write notifies all subscribers,
        // so we have nothing extra to do here. The caller pairs this with
        // `project_write_untracked` which itself only bypasses tracking; when
        // combined, the caller is responsible for using a normal write to
        // notify subscribers (see the LensOnly convention below).
    }
    fn project_mark_dirty_shallow(&self) {}
    fn project_mark_dirty_at_and_after_index(&self, _index: usize) {}
    fn project_mark_hash_child_dirty<K: std::hash::Hash + ?Sized>(&self, _key: &K) {}

    fn project_write_untracked(&self) -> WritableRef<'static, L>
    where
        L: Writable,
        L::Target: 'static,
    {
        // For LensOnly, this WILL notify subscribers via the signal's normal
        // write path (there's no separate "tracked" write). That's fine: the
        // whole point of LensOnly is to forego the path-level granularity and
        // just use the signal's own subscription list.
        self.lens.write_unchecked()
    }

    fn project_lens(&self) -> &L {
        &self.lens
    }
}

// ---------------------------------------------------------------------------
// Shape: Option<T>
// ---------------------------------------------------------------------------

/// Projection methods for types targeting `Option<T>`.
pub trait ProjectOption<T: 'static>:
    Project<Lens: Readable<Target = Option<T>>>
{
    /// Is the option currently `Some`? Tracks shallowly.
    fn is_some(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_some()
    }

    /// Is the option currently `None`? Tracks shallowly.
    fn is_none(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_none()
    }

    /// Tracks shallowly and deeply if the option is `Some`.
    fn is_some_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        if let Some(v) = &*self.project_peek() {
            self.project_track();
            f(v)
        } else {
            false
        }
    }

    /// Tracks shallowly and deeply if the option is `Some`.
    fn is_none_or(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        if let Some(v) = &*self.project_peek() {
            self.project_track();
            f(v)
        } else {
            true
        }
    }

    /// Transpose `Self<Option<T>>` into `Option<Self<T>>`.
    fn transpose(self) -> Option<Self::Child<T, fn(&Option<T>) -> &T, fn(&mut Option<T>) -> &mut T>> {
        if self.is_some() {
            let map: fn(&Option<T>) -> &T = |v| v.as_ref().unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"));
            let map_mut: fn(&mut Option<T>) -> &mut T = |v| v.as_mut().unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"));
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Unwrap to `Self<T>`; panics if currently `None`.
    fn unwrap(self) -> Self::Child<T, fn(&Option<T>) -> &T, fn(&mut Option<T>) -> &mut T> {
        self.transpose().unwrap_or_else(|| panic!("called `unwrap` on a `None` Option projection"))
    }

    /// Unwrap to `Self<T>`; panics with `msg` if currently `None`.
    fn expect(self, msg: &'static str) -> Self::Child<T, fn(&Option<T>) -> &T, fn(&mut Option<T>) -> &mut T> {
        self.transpose().unwrap_or_else(|| panic!("{}", msg))
    }

    /// Project through `Deref` on the contained value (e.g. `Option<Box<T>>` → `Option<Self<T>>`).
    fn as_deref(
        self,
    ) -> Option<Self::Child<T::Target, fn(&Option<T>) -> &T::Target, fn(&mut Option<T>) -> &mut T::Target>>
    where
        T: DerefMut,
        T::Target: 'static,
    {
        if self.is_some() {
            let map: fn(&Option<T>) -> &T::Target = |v| (&**v.as_ref().unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))) as &T::Target;
            let map_mut: fn(&mut Option<T>) -> &mut T::Target = |v| &mut **v.as_mut().unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"));
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Filter the option by a predicate. Always tracks shallowly; tracks deeply when `Some`.
    fn filter(
        self,
        f: impl FnOnce(&T) -> bool,
    ) -> Option<Self::Child<T, fn(&Option<T>) -> &T, fn(&mut Option<T>) -> &mut T>> {
        if self.is_some_and(f) {
            let map: fn(&Option<T>) -> &T = |v| v.as_ref().unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"));
            let map_mut: fn(&mut Option<T>) -> &mut T = |v| v.as_mut().unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"));
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Peek at the inner value if `Some`; tracks shallowly, and deeply when `Some`.
    fn inspect(self, f: impl FnOnce(&T)) -> Self {
        self.project_track_shallow();
        if let Some(v) = &*self.project_peek() {
            self.project_track();
            f(v);
        }
        self
    }
}

impl<T: 'static, P> ProjectOption<T> for P where P: Project<Lens: Readable<Target = Option<T>>> {}

// ---------------------------------------------------------------------------
// Shape: Result<T, E>
// ---------------------------------------------------------------------------

/// Projection methods for types targeting `Result<T, E>`.
pub trait ProjectResult<T: 'static, E: 'static>:
    Project<Lens: Readable<Target = Result<T, E>>>
{
    fn is_ok(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_ok()
    }

    fn is_err(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_err()
    }

    fn is_ok_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        match &*self.project_peek() {
            Ok(v) => {
                self.project_track();
                f(v)
            }
            Err(_) => false,
        }
    }

    fn is_err_and(&self, f: impl FnOnce(&E) -> bool) -> bool {
        self.project_track_shallow();
        match &*self.project_peek() {
            Err(e) => {
                self.project_track();
                f(e)
            }
            Ok(_) => false,
        }
    }

    #[allow(clippy::type_complexity)]
    fn ok(self) -> Option<Self::Child<T, fn(&Result<T, E>) -> &T, fn(&mut Result<T, E>) -> &mut T>> {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T = |r| r.as_ref().unwrap_or_else(|_| panic!("Tried to access `Ok` on a `Result` value"));
            let map_mut: fn(&mut Result<T, E>) -> &mut T = |r| r.as_mut().unwrap_or_else(|_| panic!("Tried to access `Ok` on a `Result` value"));
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    #[allow(clippy::type_complexity)]
    fn err(self) -> Option<Self::Child<E, fn(&Result<T, E>) -> &E, fn(&mut Result<T, E>) -> &mut E>> {
        if self.is_err() {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on a `Result` value")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on a `Result` value")
                }
            };
            Some(self.project_child(1, map, map_mut))
        } else {
            None
        }
    }

    #[allow(clippy::type_complexity)]
    fn transpose(
        self,
    ) -> Result<
        Self::Child<T, fn(&Result<T, E>) -> &T, fn(&mut Result<T, E>) -> &mut T>,
        Self::Child<E, fn(&Result<T, E>) -> &E, fn(&mut Result<T, E>) -> &mut E>,
    > {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T = |r| r.as_ref().unwrap_or_else(|_| panic!("unreachable"));
            let map_mut: fn(&mut Result<T, E>) -> &mut T = |r| r.as_mut().unwrap_or_else(|_| panic!("unreachable"));
            Ok(self.project_child(0, map, map_mut))
        } else {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("unreachable")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("unreachable")
                }
            };
            Err(self.project_child(1, map, map_mut))
        }
    }

    /// Unwrap into `Self<T>`; panics if currently `Err`.
    fn unwrap(self) -> Self::Child<T, fn(&Result<T, E>) -> &T, fn(&mut Result<T, E>) -> &mut T>
    where
        E: std::fmt::Debug,
    {
        match self.transpose() {
            Ok(ok) => ok,
            Err(_) => panic!("called `unwrap` on an Err Result projection"),
        }
    }

    /// Unwrap into `Self<T>`; panics with `msg` if currently `Err`.
    fn expect(
        self,
        msg: &'static str,
    ) -> Self::Child<T, fn(&Result<T, E>) -> &T, fn(&mut Result<T, E>) -> &mut T>
    where
        E: std::fmt::Debug,
    {
        match self.transpose() {
            Ok(ok) => ok,
            Err(_) => panic!("{}", msg),
        }
    }

    /// Unwrap into `Self<E>`; panics if currently `Ok`.
    fn unwrap_err(self) -> Self::Child<E, fn(&Result<T, E>) -> &E, fn(&mut Result<T, E>) -> &mut E>
    where
        T: std::fmt::Debug,
    {
        match self.transpose() {
            Err(e) => e,
            Ok(_) => panic!("called `unwrap_err` on an Ok Result projection"),
        }
    }

    /// Unwrap into `Self<E>`; panics with `msg` if currently `Ok`.
    fn expect_err(
        self,
        msg: &'static str,
    ) -> Self::Child<E, fn(&Result<T, E>) -> &E, fn(&mut Result<T, E>) -> &mut E>
    where
        T: std::fmt::Debug,
    {
        match self.transpose() {
            Err(e) => e,
            Ok(_) => panic!("{}", msg),
        }
    }

    /// Inspect the inner `Ok` value if present; tracks shallowly, and deeply when `Ok`.
    fn inspect(self, f: impl FnOnce(&T)) -> Self {
        self.project_track_shallow();
        if let Ok(v) = &*self.project_peek() {
            self.project_track();
            f(v);
        }
        self
    }

    /// Inspect the inner `Err` value if present; tracks shallowly, and deeply when `Err`.
    fn inspect_err(self, f: impl FnOnce(&E)) -> Self {
        self.project_track_shallow();
        if let Err(e) = &*self.project_peek() {
            self.project_track();
            f(e);
        }
        self
    }

    /// Project through `Deref` on the `Ok` / `Err` variants.
    #[allow(clippy::type_complexity)]
    fn as_deref(
        self,
    ) -> Result<
        Self::Child<T::Target, fn(&Result<T, E>) -> &T::Target, fn(&mut Result<T, E>) -> &mut T::Target>,
        Self::Child<E, fn(&Result<T, E>) -> &E, fn(&mut Result<T, E>) -> &mut E>,
    >
    where
        T: DerefMut,
        T::Target: 'static,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T::Target = |r| match r {
                Ok(t) => &**t,
                Err(_) => panic!("Tried to access `Ok` on an `Err` value"),
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T::Target = |r| match r {
                Ok(t) => &mut **t,
                Err(_) => panic!("Tried to access `Ok` on an `Err` value"),
            };
            Ok(self.project_child(0, map, map_mut))
        } else {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on an `Ok` value")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on an `Ok` value")
                }
            };
            Err(self.project_child(1, map, map_mut))
        }
    }
}

impl<T: 'static, E: 'static, P> ProjectResult<T, E> for P where P: Project<Lens: Readable<Target = Result<T, E>>> {}

// ---------------------------------------------------------------------------
// Shape: Deref (lens-level pass-through)
// ---------------------------------------------------------------------------

/// Project through a `DerefMut` target without introducing a new path subscription.
pub trait ProjectDeref<U: ?Sized + 'static>:
    Project<Lens: Readable<Target: DerefMut<Target = U>>>
{
    fn deref(self) -> Self::Child<U, fn(&<Self::Lens as Readable>::Target) -> &U, fn(&mut <Self::Lens as Readable>::Target) -> &mut U>
    where
        <Self::Lens as Readable>::Target: 'static,
    {
        let map: fn(&<Self::Lens as Readable>::Target) -> &U = |t| &**t;
        let map_mut: fn(&mut <Self::Lens as Readable>::Target) -> &mut U = |t| &mut **t;
        self.project_map(map, map_mut)
    }
}

impl<U: ?Sized + 'static, P> ProjectDeref<U> for P
where
    P: Project<Lens: Readable<Target: DerefMut<Target = U>>>,
{
}

// ---------------------------------------------------------------------------
// Shape: Vec<T> (mutation-side)
// ---------------------------------------------------------------------------

/// Mutation methods on vector-shaped projections.
pub trait ProjectVec<T: 'static>: Project<Lens: Writable<Target = Vec<T>>> {
    /// Push an item to the end.
    fn push(&self, value: T) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().push(value);
    }

    /// Remove and return the item at `index`.
    fn remove(&self, index: usize) -> T {
        self.project_mark_dirty_shallow();
        self.project_mark_dirty_at_and_after_index(index);
        self.project_write_untracked().remove(index)
    }

    /// Insert an item at `index`.
    fn insert(&self, index: usize, value: T) {
        self.project_mark_dirty_shallow();
        self.project_mark_dirty_at_and_after_index(index);
        self.project_write_untracked().insert(index, value);
    }

    /// Clear all items.
    fn clear(&self) {
        self.project_mark_dirty();
        self.project_write_untracked().clear();
    }

    /// Retain only elements for which `f` returns true.
    fn retain(&self, mut f: impl FnMut(&T) -> bool) {
        let mut index = 0;
        let mut first_removed_index: Option<usize> = None;
        self.project_write_untracked().retain(|item| {
            let keep = f(item);
            if !keep {
                first_removed_index = first_removed_index.or(Some(index));
            }
            index += 1;
            keep
        });
        if let Some(index) = first_removed_index {
            self.project_mark_dirty_shallow();
            self.project_mark_dirty_at_and_after_index(index);
        }
    }
}

impl<T: 'static, P> ProjectVec<T> for P where P: Project<Lens: Writable<Target = Vec<T>>> {}

// ---------------------------------------------------------------------------
// Shape: slice-like (Vec<T>) — read-only tracking methods
// ---------------------------------------------------------------------------

/// Read-side methods on `Vec<T>` projections (len / is_empty).
pub trait ProjectSlice<T: 'static>: Project<Lens: Readable<Target = Vec<T>>> {
    /// Length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_peek().len()
    }

    /// Is the slice empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_empty()
    }
}

impl<T: 'static, P> ProjectSlice<T> for P where P: Project<Lens: Readable<Target = Vec<T>>> {}

// ---------------------------------------------------------------------------
// Shape: HashMap<K, V, St>
// ---------------------------------------------------------------------------

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};

/// Read-side methods on `HashMap<K, V, St>` projections.
pub trait ProjectHashMap<K: 'static, V: 'static, St: 'static>:
    Project<Lens: Readable<Target = HashMap<K, V, St>>>
{
    /// Map length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_peek().len()
    }

    /// Is the map empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_empty()
    }

    /// Check whether a key exists; tracks shallowly.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.project_track_shallow();
        self.project_peek().contains_key(key)
    }
}

impl<K, V, St, P> ProjectHashMap<K, V, St> for P
where
    K: 'static,
    V: 'static,
    St: 'static,
    P: Project<Lens: Readable<Target = HashMap<K, V, St>>>,
{
}

/// Mutation methods on `HashMap<K, V, St>` projections.
pub trait ProjectHashMapMut<K: 'static, V: 'static, St: 'static>:
    Project<Lens: Writable<Target = HashMap<K, V, St>>>
{
    /// Insert a key-value pair; marks shape dirty + existing child at the hash path dirty.
    fn insert(&self, key: K, value: V)
    where
        K: Eq + Hash,
        St: BuildHasher,
    {
        self.project_mark_dirty_shallow();
        self.project_mark_hash_child_dirty(&key);
        self.project_write_untracked().insert(key, value);
    }

    /// Remove a key; marks shape dirty.
    fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Eq,
        K: Borrow<Q> + Eq + Hash,
        St: BuildHasher,
    {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().remove(key)
    }

    /// Clear the map; marks shape dirty.
    fn clear(&self) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().clear();
    }

    /// Retain only entries matching `f`; marks shape dirty.
    fn retain(&self, mut f: impl FnMut(&K, &V) -> bool) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().retain(|k, v| f(k, v));
    }
}

impl<K, V, St, P> ProjectHashMapMut<K, V, St> for P
where
    K: 'static,
    V: 'static,
    St: 'static,
    P: Project<Lens: Writable<Target = HashMap<K, V, St>>>,
{
}

// ---------------------------------------------------------------------------
// Shape: BTreeMap<K, V>
// ---------------------------------------------------------------------------

/// Read-side methods on `BTreeMap<K, V>` projections.
pub trait ProjectBTreeMap<K: 'static, V: 'static>:
    Project<Lens: Readable<Target = BTreeMap<K, V>>>
{
    /// Map length; tracks shallowly.
    fn len(&self) -> usize {
        self.project_track_shallow();
        self.project_peek().len()
    }

    /// Is the map empty? Tracks shallowly.
    fn is_empty(&self) -> bool {
        self.project_track_shallow();
        self.project_peek().is_empty()
    }

    /// Check whether a key exists; tracks shallowly.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Ord,
        K: Borrow<Q> + Ord,
    {
        self.project_track_shallow();
        self.project_peek().contains_key(key)
    }
}

impl<K, V, P> ProjectBTreeMap<K, V> for P
where
    K: 'static,
    V: 'static,
    P: Project<Lens: Readable<Target = BTreeMap<K, V>>>,
{
}

/// Mutation methods on `BTreeMap<K, V>` projections.
pub trait ProjectBTreeMapMut<K: 'static, V: 'static>:
    Project<Lens: Writable<Target = BTreeMap<K, V>>>
{
    /// Insert a key-value pair; marks shape dirty.
    fn insert(&self, key: K, value: V) -> Option<V>
    where
        K: Ord + Hash,
    {
        self.project_mark_dirty_shallow();
        self.project_mark_hash_child_dirty(&key);
        self.project_write_untracked().insert(key, value)
    }

    /// Remove a key; marks shape dirty.
    fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Ord,
        K: Borrow<Q> + Ord,
    {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().remove(key)
    }

    /// Clear the map; marks shape dirty.
    fn clear(&self) {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().clear();
    }

    /// Retain only entries matching `f`; marks shape dirty.
    fn retain(&self, mut f: impl FnMut(&K, &mut V) -> bool)
    where
        K: Ord,
    {
        self.project_mark_dirty_shallow();
        self.project_write_untracked().retain(|k, v| f(k, v));
    }
}

impl<K, V, P> ProjectBTreeMapMut<K, V> for P
where
    K: 'static,
    V: 'static,
    P: Project<Lens: Writable<Target = BTreeMap<K, V>>>,
{
}
