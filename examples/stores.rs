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

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let value = use_store(|| Value {
        count: 0,
        values: vec![Value {
            count: 0,
            values: Vec::new(),
        }],
    });

    let mut count = value().count();
    let values = value().values();

    use_effect(move || {
        // This effect will run whenever the value changes
        println!("App value changed: {}", count.read());
    });

    rsx! {
        h1 { "Counter App {count.cloned()}" }
        button { onclick: move |_| *count.write() += 1, "Up high!" }
        button { onclick: move |_| *count.write() -= 1, "Down low!" }

        button { onclick: move |_| values.push(Value{ count: 0, values: Vec::new() }), "Push child" }

        for child in values.iter() {
            Child {
                value: WriteSignal::new(child.count()),
            }
        }
    }
}

#[component]
fn Child(value: WriteSignal<u32>) -> Element {
    use_effect(move || {
        // This effect will run whenever the value changes
        println!("Child component value changed: {}", value.read());
    });
    rsx! {
        h2 { "Child component with count {value}" }
        button { onclick: move |_| value += 1, "Increment" }
        button { onclick: move |_| value -= 1, "Decrement" }
    }
}

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

#[derive(Copy, Clone, Default, PartialEq)]
struct TinyVec {
    length: usize,
    path: [u32; 16],
}

impl TinyVec {
    const fn new() -> Self {
        Self {
            length: 0,
            path: [0; 16],
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
struct StoreSubscriptions<S: Storage<SelectorNode> = UnsyncStorage> {
    root: CopyValue<SelectorNode, S>,
}

impl<S: Storage<SelectorNode>> Clone for StoreSubscriptions<S> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<S: Storage<SelectorNode>> Copy for StoreSubscriptions<S> {}

impl<S: Storage<SelectorNode>> PartialEq for StoreSubscriptions<S> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl<S: Storage<SelectorNode>> StoreSubscriptions<S> {
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

struct SelectionPath<S: Storage<SelectorNode> = UnsyncStorage> {
    path: TinyVec,
    store: StoreSubscriptions<S>,
}

impl<S: Storage<SelectorNode>> Clone for SelectionPath<S> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            store: self.store.clone(),
        }
    }
}

impl<S: Storage<SelectorNode>> Copy for SelectionPath<S> {}

impl<S: Storage<SelectorNode>> SelectionPath<S> {
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

struct SelectorScope<W, S: Storage<SelectorNode> = UnsyncStorage> {
    path: SelectionPath<S>,
    write: W,
}

impl<W, S: Storage<SelectorNode>> Clone for SelectorScope<W, S>
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

impl<W, S: Storage<SelectorNode>> Copy for SelectorScope<W, S> where W: Copy {}

impl<W, S: Storage<SelectorNode>> SelectorScope<W, S> {
    fn new(path: SelectionPath<S>, write: W) -> Self {
        Self { path, write }
    }

    fn scope<U: 'static>(
        self,
        index: u32,
        map: impl FnOnce(&W::Target) -> &U + Copy + 'static,
        map_mut: impl FnOnce(&mut W::Target) -> &mut U + Copy + 'static,
    ) -> SelectorScope<impl Writable<Target = U, Storage = W::Storage> + Copy + 'static, S>
    where
        W: Writable + Copy + 'static,
    {
        let Self { mut path, write } = self;
        path.path.push(index);
        let write = write.map_mut(move |value| map(value), move |value| map_mut(value));
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
}

impl<W: Readable, S: Storage<SelectorNode>> SelectorScope<W, S> {
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

impl<W: Writable, S: Storage<SelectorNode>> SelectorScope<W, S> {
    fn try_write_unchecked(&self) -> Result<WritableRef<'static, W>, BorrowMutError> {
        self.path.mark_dirty();
        self.write.try_write_unchecked()
    }
}

struct Store<T: 'static, S: Storage<SelectorNode> + Storage<T> = UnsyncStorage> {
    store: StoreSubscriptions<S>,
    value: CopyValue<T, S>,
}

impl<T: 'static, S: Storage<SelectorNode> + Storage<T>> PartialEq for Store<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.store == other.store && self.value == other.value
    }
}

impl<T, S: Storage<SelectorNode> + Storage<T>> Clone for Store<T, S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T, S: Storage<SelectorNode> + Storage<T>> Copy for Store<T, S> where T: 'static {}

impl<T: Selectable, S: Storage<SelectorNode> + Storage<T>> Store<T, S> {
    fn select(&self) -> T::Selector<CopyValue<T, S>, S> {
        let path = SelectionPath::new(self.store.clone());
        let selector = SelectorScope {
            path,
            write: self.value,
        };
        T::Selector::new(selector)
    }
}

impl<T: Selectable, S: Storage<SelectorNode> + Storage<T>> Deref for Store<T, S> {
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

fn create_maybe_sync_store<T: Selectable, S: Storage<SelectorNode> + Storage<T>>(
    value: T,
) -> Store<T, S> {
    let store = StoreSubscriptions::new();
    let value = CopyValue::new_maybe_sync(value);
    Store { store, value }
}

fn use_maybe_sync_store<T: Selectable, S: Storage<SelectorNode> + Storage<T>>(
    init: impl FnOnce() -> T,
) -> Store<T, S> {
    use_hook(move || create_maybe_sync_store(init()))
}

fn create_store<T: Selectable>(value: T) -> Store<T, UnsyncStorage> {
    create_maybe_sync_store::<T, UnsyncStorage>(value)
}

fn use_store<T: Selectable>(init: impl FnOnce() -> T) -> Store<T, UnsyncStorage> {
    use_hook(move || create_store(init()))
}

trait Selectable {
    type Selector<View, Storage: dioxus::prelude::Storage<SelectorNode>>: CreateSelector<
        View = View,
        Storage = Storage,
    >;
}

trait CreateSelector {
    type View;
    type Storage: Storage<SelectorNode>;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self;
}

struct Value {
    count: u32,
    values: Vec<Value>,
}

impl Selectable for Value {
    type Selector<View, Storage: dioxus::prelude::Storage<SelectorNode>> =
        ValueSelector<View, Storage>;
}

struct ValueSelector<W, S: Storage<SelectorNode> = UnsyncStorage> {
    selector: SelectorScope<W, S>,
}

impl<W, S: Storage<SelectorNode>> CreateSelector for ValueSelector<W, S> {
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self { selector }
    }
}

impl<W: Writable<Target = Value, Storage = S> + Copy + 'static, S: Storage<SelectorNode>>
    ValueSelector<W, S>
{
    fn count(self) -> TSelector<impl Writable<Target = u32, Storage = S> + Copy + 'static, u32, S> {
        TSelector::new(
            self.selector
                .scope(0, |value| &value.count, |value| &mut value.count),
        )
    }

    fn values(
        self,
    ) -> VecSelector<impl Writable<Target = Vec<Value>, Storage = S> + Copy + 'static, Value, S>
    {
        VecSelector::new(
            self.selector
                .scope(1, |value| &value.values, |value| &mut value.values),
        )
    }
}

impl<T> Selectable for Vec<T> {
    type Selector<View, Storage: dioxus::prelude::Storage<SelectorNode>> =
        VecSelector<View, T, Storage>;
}

struct VecSelector<W, T, S: Storage<SelectorNode> = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T, S: Storage<SelectorNode>> Clone for VecSelector<W, T, S>
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

impl<W, T, S: Storage<SelectorNode>> Copy for VecSelector<W, T, S> where W: Copy {}

impl<W, T, S: Storage<SelectorNode>> CreateSelector for VecSelector<W, T, S> {
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
        S: Storage<SelectorNode>,
    > VecSelector<W, T, S>
{
    fn index(
        self,
        index: u32,
    ) -> T::Selector<impl Writable<Target = T, Storage = S> + Copy + 'static, S> {
        T::Selector::new(self.selector.scope(
            index,
            move |value| &value[index as usize],
            move |value| &mut value[index as usize],
        ))
    }

    fn len(self) -> usize {
        self.selector.track();
        self.selector.write.read().len()
    }

    fn is_empty(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_empty()
    }

    fn iter(
        self,
    ) -> impl Iterator<Item = T::Selector<impl Writable<Target = T, Storage = S> + Copy + 'static, S>>
    {
        (0..self.len()).map(move |i| self.index(i as u32))
    }

    fn push(self, value: T) {
        self.selector.mark_dirty_shallow();
        self.selector.write.write_unchecked().push(value);
    }
}

struct TSelector<W, T, S: Storage<SelectorNode> = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    phantom: PhantomData<T>,
}

impl<W, T, S: Storage<SelectorNode>> Clone for TSelector<W, T, S>
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

impl<W, T, S: Storage<SelectorNode>> Copy for TSelector<W, T, S> where W: Copy {}

impl<W: Writable<Target = T> + Copy + 'static, T, S: Storage<SelectorNode>> CreateSelector
    for TSelector<W, T, S>
{
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self {
            selector,
            phantom: PhantomData,
        }
    }
}

impl<W, T: 'static, S: Storage<SelectorNode>> Readable for TSelector<W, T, S>
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

impl<W, T: 'static, S: Storage<SelectorNode>> Writable for TSelector<W, T, S>
where
    W: Writable<Target = T>,
{
    type WriteMetadata = <W as Writable>::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}
