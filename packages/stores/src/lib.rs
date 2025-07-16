//! The example from the readme!
//!
//! This example demonstrates how to create a simple counter app with dioxus. The `Signal` type wraps inner values,
//! making them `Copy`, allowing them to be freely used in closures and async functions. `Signal` also provides
//! helper methods like AddAssign, SubAssign, toggle, etc, to make it easy to update the value without running
//! into lock issues.

use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    mem::MaybeUninit,
    ops::Deref,
    sync::{Arc, Mutex},
};

use dioxus_core::{prelude::ReactiveContext, use_hook};
use dioxus_signals::{
    BorrowError, BorrowMutError, CopyValue, MappedMutSignal, Readable, ReadableExt, ReadableRef,
    Storage, Subscribers, UnsyncStorage, Writable, WritableExt, WritableRef, WriteSignal,
};

// Re-exported for the macro
#[doc(hidden)]
pub mod macro_helpers {
    pub use dioxus_signals;
}

#[allow(private_bounds)]
pub trait SelectorStorage: Storage<SelectorNode> {}
impl<S: Storage<SelectorNode>> SelectorStorage for S {}

#[derive(Clone, Default)]
struct SelectorNode {
    subscribers: Arc<Mutex<HashSet<ReactiveContext>>>,
    root: HashMap<u32, SelectorNode>,
}

impl SelectorNode {
    fn find(&self, path: &[u32]) -> Option<&SelectorNode> {
        let [first, rest @ ..] = path else {
            return Some(self);
        };
        self.root.get(&first).and_then(|child| child.find(rest))
    }

    fn find_parent(&self, path: &[u32]) -> Option<&SelectorNode> {
        match path {
            [] => None,
            [_] => return Some(self),
            [rest @ .., _last] => self.find(rest),
        }
    }

    fn read(&mut self, path: &[u32]) {
        let [first, rest @ ..] = path else {
            if let Some(rc) = ReactiveContext::current() {
                rc.subscribe(self.subscribers.clone());
            }
            return;
        };
        self.root.entry(*first).or_default().read(rest);
    }

    fn visit_depth_first(&self, f: &mut dyn FnMut(&SelectorNode)) {
        f(self);
        for child in self.root.values() {
            child.visit_depth_first(&mut *f);
        }
    }

    fn write(&self, path: &[u32]) {
        let Some(node) = self.find(path) else {
            return;
        };

        // Mark the node and all its children as dirty
        node.visit_depth_first(&mut |node| {
            node.mark_dirty();
        });
    }

    fn mark_dirty_shallow(&self, path: &[u32]) {
        let Some(node) = self.find(path) else {
            return;
        };

        // Mark the node as dirty
        node.mark_dirty();
    }

    fn mark_larger_dirty(&self, path: &[u32]) {
        let Some(last) = path.last().copied() else {
            return;
        };
        let Some(node) = self.find_parent(path) else {
            return;
        };

        for (key, larger) in node.root.iter() {
            if *key < last {
                continue;
            }
            // Mark all larger nodes as dirty
            larger.visit_depth_first(&mut |node| {
                node.mark_dirty();
            });
        }
    }

    fn mark_dirty(&self) {
        // We cannot hold the subscribers lock while calling mark_dirty, because mark_dirty can run user code which may cause a new subscriber to be added. If we hold the lock, we will deadlock.
        #[allow(clippy::mutable_key_type)]
        let mut subscribers = std::mem::take(&mut *self.subscribers.lock().unwrap());
        subscribers.retain(|reactive_context| reactive_context.mark_dirty());
        // Extend the subscribers list instead of overwriting it in case a subscriber is added while reactive contexts are marked dirty
        self.subscribers.lock().unwrap().extend(subscribers);
    }
}

#[derive(Copy, Clone, PartialEq)]
struct TinyVec {
    length: usize,
    path: [u32; 64],
}

impl Default for TinyVec {
    fn default() -> Self {
        Self::new()
    }
}

impl TinyVec {
    const fn new() -> Self {
        Self {
            length: 0,
            path: [0; 64],
        }
    }

    pub const fn push(&mut self, index: u32) {
        if self.length < self.path.len() {
            self.path[self.length] = index;
            self.length += 1;
        } else {
            panic!("SelectorPath is full");
        }
    }
}

impl Deref for TinyVec {
    type Target = [u32];

    fn deref(&self) -> &Self::Target {
        &self.path[..self.length]
    }
}

#[derive(Default)]
struct StoreSubscriptions<S: SelectorStorage = UnsyncStorage> {
    root: CopyValue<SelectorNode, S>,
}

impl<S: SelectorStorage> Clone for StoreSubscriptions<S> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<S: SelectorStorage> Copy for StoreSubscriptions<S> {}

impl<S: SelectorStorage> PartialEq for StoreSubscriptions<S> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl<S: SelectorStorage> StoreSubscriptions<S> {
    fn new() -> Self {
        Self {
            root: CopyValue::new_maybe_sync(SelectorNode::default()),
        }
    }

    fn read(&self, key: &[u32]) {
        self.root.write_unchecked().read(key);
    }

    fn write(&self, key: &[u32]) {
        self.root.read().write(key);
    }

    fn mark_dirty_shallow(&self, key: &[u32]) {
        self.root.read().mark_dirty_shallow(key);
    }

    fn subscribers(&self, key: &[u32]) -> Option<Subscribers> {
        let read = self.root.read();
        let node = read.find(key)?;
        Some(node.subscribers.clone())
    }
}

struct SelectionPath<S: SelectorStorage = UnsyncStorage> {
    path: TinyVec,
    store: StoreSubscriptions<S>,
}

impl<S: SelectorStorage> PartialEq for SelectionPath<S> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.store == other.store
    }
}

impl<S: SelectorStorage> Clone for SelectionPath<S> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            store: self.store.clone(),
        }
    }
}

impl<S: SelectorStorage> Copy for SelectionPath<S> {}

impl<S: SelectorStorage> SelectionPath<S> {
    fn new(store: StoreSubscriptions<S>) -> Self {
        Self {
            path: TinyVec::new(),
            store,
        }
    }

    fn track(&self) {
        self.store.read(&*self.path);
    }

    fn mark_dirty(&self) {
        self.store.write(&*self.path);
    }

    fn mark_dirty_shallow(&self) {
        self.store.mark_dirty_shallow(&*self.path);
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.store.subscribers(&*self.path)
    }
}

pub struct SelectorScope<W, S: SelectorStorage = UnsyncStorage> {
    path: SelectionPath<S>,
    write: W,
}

impl<W: PartialEq, S: SelectorStorage> PartialEq for SelectorScope<W, S> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.write == other.write
    }
}

impl<W, S: SelectorStorage> Clone for SelectorScope<W, S>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            write: self.write.clone(),
        }
    }
}

impl<W, S: SelectorStorage> Copy for SelectorScope<W, S> where W: Copy {}

impl<W, S: SelectorStorage> SelectorScope<W, S> {
    fn new(path: SelectionPath<S>, write: W) -> Self {
        Self { path, write }
    }

    pub fn scope<U: 'static, F, FMut>(
        self,
        index: u32,
        map: F,
        map_mut: FMut,
    ) -> SelectorScope<MappedMutSignal<U, W, F, FMut>, S>
    where
        W: Writable<Storage = S> + Copy + 'static,
        F: Fn(&W::Target) -> &U + Copy + 'static,
        FMut: Fn(&mut W::Target) -> &mut U + Copy + 'static,
    {
        let Self { mut path, write } = self;
        path.path.push(index);
        let write = write.map_mut(map, map_mut);
        SelectorScope::new(path, write)
    }

    fn track(&self) {
        self.path.track();
    }

    fn mark_dirty(&self) {
        self.path.mark_dirty();
    }

    fn mark_dirty_shallow(&self) {
        self.path.mark_dirty_shallow();
    }

    /// Map the writer to a new type.
    pub fn map<W2>(self, map: impl FnOnce(W) -> W2) -> SelectorScope<W2, S> {
        SelectorScope {
            path: self.path,
            write: map(self.write),
        }
    }
}

impl<W: Readable, S: SelectorStorage> SelectorScope<W, S> {
    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.track();
        self.write.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, W>, BorrowError> {
        self.write.try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.path.subscribers()
    }
}

impl<W: Writable, S: SelectorStorage> SelectorScope<W, S> {
    fn try_write_unchecked(&self) -> Result<WritableRef<'static, W>, BorrowMutError> {
        self.path.mark_dirty();
        self.write.try_write_unchecked()
    }
}

pub type Selector<T, W = WriteSignal<T>, S = UnsyncStorage> = <T as Selectable>::Selector<W, S>;

pub struct Store<T: 'static, S: SelectorStorage + Storage<T> = UnsyncStorage> {
    store: StoreSubscriptions<S>,
    value: CopyValue<T, S>,
}

impl<T: 'static, S: SelectorStorage + Storage<T>> PartialEq for Store<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.store == other.store && self.value == other.value
    }
}

impl<T, S: SelectorStorage + Storage<T>> Clone for Store<T, S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T, S: SelectorStorage + Storage<T>> Copy for Store<T, S> where T: 'static {}

impl<T: Selectable, S: SelectorStorage + Storage<T>> Store<T, S> {
    fn select(&self) -> T::Selector<CopyValue<T, S>, S> {
        let path = SelectionPath::new(self.store.clone());
        let selector = SelectorScope {
            path,
            write: self.value,
        };
        T::Selector::new(selector)
    }
}

impl<T: Selectable, S: SelectorStorage + Storage<T>> Deref for Store<T, S> {
    type Target = dyn Fn() -> T::Selector<CopyValue<T, S>, S>;

    fn deref(&self) -> &Self::Target {
        // https://github.com/dtolnay/case-studies/tree/master/callable-types

        // First we create a closure that captures something with the Same in memory layout as Self (MaybeUninit<Self>).
        let uninit_callable = MaybeUninit::<Self>::uninit();
        // Then move that value into the closure. We assume that the closure now has a in memory layout of Self.
        let uninit_closure = move || Self::select(unsafe { &*uninit_callable.as_ptr() });

        // Check that the size of the closure is the same as the size of Self in case the compiler changed the layout of the closure.
        let size_of_closure = std::mem::size_of_val(&uninit_closure);
        assert_eq!(size_of_closure, std::mem::size_of::<Self>());

        // Then cast the lifetime of the closure to the lifetime of &self.
        fn cast_lifetime<'a, T>(_a: &T, b: &'a T) -> &'a T {
            b
        }
        let reference_to_closure = cast_lifetime(
            {
                // The real closure that we will never use.
                &uninit_closure
            },
            #[allow(clippy::missing_transmute_annotations)]
            // We transmute self into a reference to the closure. This is safe because we know that the closure has the same memory layout as Self so &Closure == &Self.
            unsafe {
                std::mem::transmute(self)
            },
        );

        // Cast the closure to a trait object.
        reference_to_closure as &_
    }
}

pub fn create_maybe_sync_store<T: Selectable, S: SelectorStorage + Storage<T>>(
    value: T,
) -> Store<T, S> {
    let store = StoreSubscriptions::new();
    let value = CopyValue::new_maybe_sync(value);
    Store { store, value }
}

pub fn use_maybe_sync_store<T: Selectable, S: SelectorStorage + Storage<T>>(
    init: impl Fn() -> T,
) -> Store<T, S> {
    use_hook(move || create_maybe_sync_store(init()))
}

pub fn create_store<T: Selectable>(value: T) -> Store<T, UnsyncStorage> {
    create_maybe_sync_store::<T, UnsyncStorage>(value)
}

pub fn use_store<T: Selectable>(init: impl Fn() -> T) -> Store<T, UnsyncStorage> {
    use_hook(move || create_store(init()))
}

pub trait Selectable {
    type Selector<View, S: SelectorStorage>: CreateSelector<View = View, Storage = S>;
}

pub trait CreateSelector {
    type View;
    type Storage: SelectorStorage;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self;
}

impl<T> Selectable for Vec<T> {
    type Selector<View, S: SelectorStorage> = VecSelector<View, T, S>;
}

pub struct VecSelector<W, T, S: SelectorStorage = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T, S: SelectorStorage> Clone for VecSelector<W, T, S>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<W, T, S: SelectorStorage> Copy for VecSelector<W, T, S> where W: Copy {}

impl<W, T, S: SelectorStorage> CreateSelector for VecSelector<W, T, S> {
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<
        W: Writable<Target = Vec<T>, Storage = S> + Copy + 'static,
        T: Selectable + 'static,
        S: SelectorStorage,
    > VecSelector<W, T, S>
{
    pub fn index(
        self,
        index: u32,
    ) -> T::Selector<
        MappedMutSignal<
            T,
            W,
            impl Fn(&Vec<T>) -> &T + Copy + 'static,
            impl Fn(&mut Vec<T>) -> &mut T + Copy + 'static,
        >,
        S,
    > {
        T::Selector::new(self.selector.scope(
            index,
            move |value| &value[index as usize],
            move |value| &mut value[index as usize],
        ))
    }

    pub fn len(self) -> usize {
        self.selector.track();
        self.selector.write.read().len()
    }

    pub fn is_empty(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_empty()
    }

    pub fn iter(
        self,
    ) -> impl Iterator<
        Item = T::Selector<
            MappedMutSignal<
                T,
                W,
                impl Fn(&Vec<T>) -> &T + Copy + 'static,
                impl Fn(&mut Vec<T>) -> &mut T + Copy + 'static,
            >,
            S,
        >,
    > {
        (0..self.len()).map(move |i| self.index(i as u32))
    }

    pub fn push(self, value: T) {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().push(value);
    }
}

pub struct ForeignType<T, S: SelectorStorage = UnsyncStorage> {
    phantom: PhantomData<(T, S)>,
}

impl<T, S: SelectorStorage> Selectable for ForeignType<T, S> {
    type Selector<View, St: SelectorStorage> = TSelector<View, T, St>;
}

pub struct TSelector<W, T, S: SelectorStorage = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    phantom: PhantomData<T>,
}

impl<W, T, S: SelectorStorage> Clone for TSelector<W, T, S>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            phantom: PhantomData,
        }
    }
}

impl<W, T, S: SelectorStorage> Copy for TSelector<W, T, S> where W: Copy {}

impl<W, T, S: SelectorStorage> CreateSelector for TSelector<W, T, S> {
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self {
            selector,
            phantom: PhantomData,
        }
    }
}

impl<W, T: 'static, S: SelectorStorage> Readable for TSelector<W, T, S>
where
    W: Readable<Target = T>,
{
    type Target = T;

    type Storage = W::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.selector.subscribers()
    }
}

impl<W, T: 'static, S: SelectorStorage> Writable for TSelector<W, T, S>
where
    W: Writable<Target = T>,
{
    type WriteMetadata = <W as Writable>::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}
