//! Stores, lenses, and the `Deref`/`DerefMut`-tracking guards.

use std::any::Any;
use std::cell::{Cell, Ref, RefMut};
use std::fmt;
use std::marker::PhantomData;

use crate::runtime::{NodeId, Path, with_rt};

/// Anything that can be read through the lens machinery: a [`Store`], a
/// [`Lens`], an [`IndexLens`], a [`crate::Memo`], or a [`crate::Resource`].
///
/// Reads are zero-copy: [`Readable::read`] hands out a guard that projects
/// `&Self::Target` straight out of the underlying cell with `Ref::map`.
/// Subscription happens lazily on the guard's first `Deref`, mirroring qk's
/// `RwTrack`.
pub trait Readable: Copy + 'static {
    type Target: ?Sized + 'static;

    #[doc(hidden)]
    fn node(&self) -> NodeId;

    #[doc(hidden)]
    fn push_path(&self, out: &mut Path);

    #[doc(hidden)]
    fn project(&self, any: Ref<'static, dyn Any>) -> Ref<'static, Self::Target>;

    /// Read the value. If the current code is running inside an effect, memo,
    /// or resource, dereferencing the guard subscribes it to this exact lens
    /// path. Derived values (memos) are brought up to date *before* the guard
    /// is created, so recomputation can never invalidate a live reference.
    fn read(&self) -> ReadGuard<Self::Target> {
        with_rt(|rt| rt.ensure(self.node()));
        let mut path = Path::new();
        self.push_path(&mut path);
        let observer = with_rt(|rt| rt.current_observer());
        let inner = self.project(with_rt(|rt| rt.borrow_value(self.node())));
        ReadGuard {
            inner,
            node: self.node(),
            path,
            observer,
            tracked: Cell::new(false),
        }
    }

    /// Read without subscribing, even inside an effect.
    fn peek(&self) -> ReadGuard<Self::Target> {
        with_rt(|rt| rt.ensure(self.node()));
        let mut path = Path::new();
        self.push_path(&mut path);
        let inner = self.project(with_rt(|rt| rt.borrow_value(self.node())));
        ReadGuard {
            inner,
            node: self.node(),
            path,
            observer: None,
            tracked: Cell::new(false),
        }
    }

    /// Run `f` with a tracked reference to the value.
    fn with<R>(&self, f: impl FnOnce(&Self::Target) -> R) -> R {
        f(&self.read())
    }

    /// Copy the value out. Prefer [`Readable::read`] for zero-copy access.
    fn get(&self) -> Self::Target
    where
        Self::Target: Sized + Clone,
    {
        self.read().clone()
    }

    /// Lens into a field. `segment` is a structural index that must be unique
    /// per field within one level (use the [`crate::lens!`] macro to write
    /// these ergonomically).
    fn select<U: ?Sized + 'static>(
        self,
        segment: u32,
        map: fn(&Self::Target) -> &U,
        map_mut: fn(&mut Self::Target) -> &mut U,
    ) -> Lens<Self, U> {
        Lens {
            parent: self,
            segment,
            map,
            map_mut,
        }
    }

    /// Lens into a collection element by index.
    fn index<U>(self, index: usize) -> IndexLens<Self, U>
    where
        Self::Target: std::ops::IndexMut<usize, Output = U> + Sized,
        U: 'static,
    {
        IndexLens {
            parent: self,
            index,
            _marker: PhantomData,
        }
    }
}

/// A [`Readable`] whose root is a [`Store`], so it can also be written.
pub trait Writable: Readable {
    #[doc(hidden)]
    fn project_mut(&self, any: RefMut<'static, dyn Any>) -> RefMut<'static, Self::Target>;

    /// Get a write guard. Subscribers at overlapping paths are notified when
    /// the guard drops — but only if `DerefMut` was actually used, so reading
    /// through a `WriteGuard` never causes spurious invalidation.
    fn write(&self) -> WriteGuard<Self::Target> {
        let mut path = Path::new();
        self.push_path(&mut path);
        let observer = with_rt(|rt| rt.current_observer());
        let inner = self.project_mut(with_rt(|rt| rt.borrow_value_mut(self.node())));
        WriteGuard {
            inner: Some(inner),
            node: self.node(),
            path,
            observer,
            read_tracked: Cell::new(false),
            wrote: Cell::new(false),
        }
    }

    /// Replace the value.
    fn set(&self, value: Self::Target)
    where
        Self::Target: Sized,
    {
        *self.write() = value;
    }

    /// Run `f` with a mutable reference to the value.
    fn with_mut<R>(&self, f: impl FnOnce(&mut Self::Target) -> R) -> R {
        f(&mut self.write())
    }
}

/// The root reactive primitive: a heap cell holding a plain Rust value.
///
/// `Store<T>` is `Copy`; it is a generational id into the thread-local
/// runtime. Stores created inside an effect are owned by that effect and are
/// dropped when it reruns or unmounts; stores created at the top level live
/// for the lifetime of the thread.
pub struct Store<T: 'static> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Store<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Store<T> {}

impl<T: 'static> Store<T> {
    pub fn new(value: T) -> Self {
        let id = with_rt(|rt| {
            let id = rt.create_node(Some(Box::new(std::cell::RefCell::new(value))), None, false);
            rt.mark_clean(id);
            id
        });
        Store {
            id,
            _marker: PhantomData,
        }
    }

    /// Whether the store's owning scope is still alive.
    pub fn is_alive(&self) -> bool {
        with_rt(|rt| rt.is_alive(self.id))
    }
}

impl<T: 'static> Readable for Store<T> {
    type Target = T;

    fn node(&self) -> NodeId {
        self.id
    }

    fn push_path(&self, _out: &mut Path) {}

    fn project(&self, any: Ref<'static, dyn Any>) -> Ref<'static, T> {
        Ref::map(any, |any| {
            any.downcast_ref::<T>().expect("store type mismatch")
        })
    }
}

impl<T: 'static> Writable for Store<T> {
    fn project_mut(&self, any: RefMut<'static, dyn Any>) -> RefMut<'static, T> {
        RefMut::map(any, |any| {
            any.downcast_mut::<T>().expect("store type mismatch")
        })
    }
}

/// A zero-copy, `Copy` projection into part of a parent [`Readable`].
/// Composition is type-level (`Lens<Lens<Store<App>, Todos>, Todo>`), so no
/// allocation or boxing is involved.
pub struct Lens<P: Readable, T: ?Sized + 'static> {
    parent: P,
    segment: u32,
    map: fn(&P::Target) -> &T,
    map_mut: fn(&mut P::Target) -> &mut T,
}

impl<P: Readable, T: ?Sized + 'static> Clone for Lens<P, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: Readable, T: ?Sized + 'static> Copy for Lens<P, T> {}

impl<P: Readable, T: ?Sized + 'static> Readable for Lens<P, T> {
    type Target = T;

    fn node(&self) -> NodeId {
        self.parent.node()
    }

    fn push_path(&self, out: &mut Path) {
        self.parent.push_path(out);
        out.push(self.segment);
    }

    fn project(&self, any: Ref<'static, dyn Any>) -> Ref<'static, T> {
        Ref::map(self.parent.project(any), self.map)
    }
}

impl<P: Writable, T: ?Sized + 'static> Writable for Lens<P, T> {
    fn project_mut(&self, any: RefMut<'static, dyn Any>) -> RefMut<'static, T> {
        RefMut::map(self.parent.project_mut(any), self.map_mut)
    }
}

/// A lens into a collection element. The element index is the path segment,
/// so writes through `store.index(3)` never wake subscribers of
/// `store.index(4)`, while writes to the whole collection wake both.
pub struct IndexLens<P: Readable, T: 'static> {
    parent: P,
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<P: Readable, T: 'static> Clone for IndexLens<P, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P: Readable, T: 'static> Copy for IndexLens<P, T> {}

impl<P, T> Readable for IndexLens<P, T>
where
    P: Readable,
    P::Target: std::ops::IndexMut<usize, Output = T> + Sized,
    T: 'static,
{
    type Target = T;

    fn node(&self) -> NodeId {
        self.parent.node()
    }

    fn push_path(&self, out: &mut Path) {
        self.parent.push_path(out);
        out.push(self.index as u32);
    }

    fn project(&self, any: Ref<'static, dyn Any>) -> Ref<'static, T> {
        let index = self.index;
        Ref::map(self.parent.project(any), move |parent| &parent[index])
    }
}

impl<P, T> Writable for IndexLens<P, T>
where
    P: Writable,
    P::Target: std::ops::IndexMut<usize, Output = T> + Sized,
    T: 'static,
{
    fn project_mut(&self, any: RefMut<'static, dyn Any>) -> RefMut<'static, T> {
        let index = self.index;
        RefMut::map(self.parent.project_mut(any), move |parent| {
            &mut parent[index]
        })
    }
}

/// A zero-copy read guard. `Deref` subscribes the observer that created it
/// (once), exactly like qk's `RwTrack` marks its read bit on deref.
pub struct ReadGuard<T: ?Sized + 'static> {
    inner: Ref<'static, T>,
    node: NodeId,
    path: Path,
    observer: Option<NodeId>,
    tracked: Cell<bool>,
}

impl<T: ?Sized + 'static> ReadGuard<T> {
    /// Project the guard to a part of the value, keeping the subscription.
    pub fn map<U: ?Sized>(guard: Self, f: impl FnOnce(&T) -> &U) -> ReadGuard<U> {
        guard.track();
        ReadGuard {
            inner: Ref::map(guard.inner, f),
            node: guard.node,
            path: guard.path.clone(),
            observer: None,
            tracked: Cell::new(true),
        }
    }

    /// Project the guard to a part of the value that may not exist (e.g. an
    /// enum variant's payload), keeping the subscription.
    pub fn filter_map<U: ?Sized>(
        guard: Self,
        f: impl FnOnce(&T) -> Option<&U>,
    ) -> Option<ReadGuard<U>> {
        guard.track();
        let node = guard.node;
        let path = guard.path.clone();
        Ref::filter_map(guard.inner, f).ok().map(|inner| ReadGuard {
            inner,
            node,
            path,
            observer: None,
            tracked: Cell::new(true),
        })
    }

    fn track(&self) {
        if let Some(observer) = self.observer
            && !self.tracked.replace(true)
        {
            with_rt(|rt| rt.track_read(observer, self.node, &self.path));
        }
    }
}

impl<T: ?Sized + 'static> std::ops::Deref for ReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.track();
        &self.inner
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for ReadGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for ReadGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

/// A zero-copy write guard. `Deref` read-subscribes; `DerefMut` flags the
/// write. Subscribers are only notified on drop if `DerefMut` was used.
pub struct WriteGuard<T: ?Sized + 'static> {
    inner: Option<RefMut<'static, T>>,
    node: NodeId,
    path: Path,
    observer: Option<NodeId>,
    read_tracked: Cell<bool>,
    wrote: Cell<bool>,
}

impl<T: ?Sized + 'static> std::ops::Deref for WriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if let Some(observer) = self.observer
            && !self.read_tracked.replace(true)
        {
            with_rt(|rt| rt.track_read(observer, self.node, &self.path));
        }
        self.inner.as_ref().expect("write guard already dropped")
    }
}

impl<T: ?Sized + 'static> std::ops::DerefMut for WriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.wrote.set(true);
        self.inner.as_mut().expect("write guard already dropped")
    }
}

impl<T: ?Sized + 'static> Drop for WriteGuard<T> {
    fn drop(&mut self) {
        // Release the RefMut *before* notifying, so effects that run during
        // the flush can read the value.
        self.inner = None;
        if self.wrote.get() {
            with_rt(|rt| rt.notify_write(self.node, &self.path));
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for WriteGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

/// Ergonomic field lenses: `lens!(store => 0: field_a)` expands to
/// `store.select(0, |v| &v.field_a, |v| &mut v.field_a)`. Chains are allowed:
/// `lens!(store => 0: user, 1: name)`.
#[macro_export]
macro_rules! lens {
    ($parent:expr => $segment:literal : $field:ident $(, $($rest:tt)*)?) => {{
        let lens = $crate::Readable::select(
            $parent,
            $segment,
            |value| &value.$field,
            |value| &mut value.$field,
        );
        $crate::lens!(@chain lens $(, $($rest)*)?)
    }};
    (@chain $lens:ident) => { $lens };
    (@chain $lens:ident,) => { $lens };
    (@chain $lens:ident, $segment:literal : $field:ident $(, $($rest:tt)*)?) => {{
        let lens = $crate::Readable::select(
            $lens,
            $segment,
            |value| &value.$field,
            |value| &mut value.$field,
        );
        $crate::lens!(@chain lens $(, $($rest)*)?)
    }};
}

/// Read without tracking anywhere in `f`, even inside an effect.
pub fn untracked<R>(f: impl FnOnce() -> R) -> R {
    with_rt(|rt| rt.untracked(f))
}
