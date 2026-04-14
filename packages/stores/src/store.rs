use crate::{
    scope::SelectorScope,
    subscriptions::{StoreSubscriptions, TinyVec},
};
use dioxus_core::{
    use_hook, AttributeValue, DynamicNode, IntoAttributeValue, IntoDynNode, Subscribers, SuperInto,
};
use dioxus_signals::{
    read_impls, write_impls, BorrowError, BorrowMutError, BoxedSignalStorage, CopyValue,
    CreateBoxedSignalStorage, Global, InitializeFromFunction, MappedMutSignal, ReadSignal,
    Readable, ReadableExt, ReadableRef, Storage, SyncStorage, UnsyncStorage, Writable, WritableExt,
    WritableRef, WriteSignal,
};
use std::marker::PhantomData;

/// A type alias for a store that has been mapped with a function
pub(crate) type MappedStore<
    T,
    Lens,
    F = fn(&<Lens as Readable>::Target) -> &T,
    FMut = fn(&mut <Lens as Readable>::Target) -> &mut T,
> = Store<T, MappedMutSignal<T, Lens, F, FMut>>;

/// A type alias for a boxed read-only store.
pub type ReadStore<T, S = UnsyncStorage> = Store<T, ReadSignal<T, S>>;

/// A type alias for a boxed writable-only store.
pub type WriteStore<T, S = UnsyncStorage> = Store<T, WriteSignal<T, S>>;

/// A type alias for a store backed by SyncStorage.
pub type SyncStore<T> = Store<T, CopyValue<T, SyncStorage>>;

/// Stores are a reactive type built for nested data structures. Each store will lazily create signals
/// for each field/member of the data structure as needed.
///
/// By default stores act a lot like [`dioxus_signals::Signal`]s, but they provide more granular
/// subscriptions without requiring nested signals. You should derive [`Store`](dioxus_stores_macro::Store) on your data
/// structures to generate selectors that let you scope the store to a specific part of your data.
///
/// You can also use the [`#[store]`](dioxus_stores_macro::store) macro on an impl block to add any additional methods to your store
/// with an extension trait. This lets you add methods to the store even though the type is not defined in your crate.
///
/// # Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// fn main() {
///     dioxus::launch(app);
/// }
///
/// // Deriving the store trait provides methods to scope the store to specific parts of your data structure.
/// // The `Store` macro generates a `count` and `children` method for the `CounterTree` struct.
/// #[derive(Store, Default)]
/// struct CounterTree {
///     count: i32,
///     children: Vec<CounterTree>,
/// }
///
/// // The store macro generates an extension trait with additional methods for the store based on the impl block.
/// #[store]
/// impl<Lens> Store<CounterTree, Lens> {
///     // Methods that take &self automatically require the lens to implement `Readable` which lets you read the store.
///     fn sum(&self) -> i32 {
///        self.count().cloned() + self.children().iter().map(|c| c.sum()).sum::<i32>()
///     }
/// }
///
/// fn app() -> Element {
///     let value = use_store(Default::default);
///
///     rsx! {
///         Tree {
///             value
///         }
///     }
/// }
///
/// #[component]
/// fn Tree(value: Store<CounterTree>) -> Element {
///     // Calling the generated `count` method returns a new store that can only
///     // read and write the count field
///     let mut count = value.count();
///     let mut children = value.children();
///     rsx! {
///         button {
///             // Incrementing the count will only rerun parts of the app that have read the count field
///             onclick: move |_| count += 1,
///             "Increment"
///         }
///         button {
///             // Stores are aware of data structures like `Vec` and `Hashmap`. When we push an item to the vec
///             // it will only rerun the parts of the app that depend on the length of the vec
///             onclick: move |_| children.push(Default::default()),
///             "Push child"
///         }
///         "sum: {value.sum()}"
///         ul {
///             // Iterating over the children gives us stores scoped to each child.
///             for value in children.iter() {
///                 li {
///                     Tree { value }
///                 }
///             }
///         }
///     }
/// }
/// ```
pub struct Store<T: ?Sized, Lens = WriteSignal<T>> {
    selector: SelectorScope<Lens>,
    _phantom: PhantomData<Box<T>>,
}

impl<T: 'static, S: Storage<T>> Store<T, CopyValue<T, S>> {
    /// Creates a new `Store` that might be sync. This allocates memory in the current scope, so this should only be called
    /// inside of an initialization closure like the closure passed to [`use_hook`].
    #[track_caller]
    pub fn new_maybe_sync(value: T) -> Self {
        let store = StoreSubscriptions::new();
        let value = CopyValue::new_maybe_sync(value);

        let path = TinyVec::new();
        let selector = SelectorScope::new(path, store, value);
        selector.into()
    }
}

impl<T: 'static> Store<T> {
    /// Creates a new `Store`. This allocates memory in the current scope, so this should only be called
    /// inside of an initialization closure like the closure passed to [`use_hook`].
    #[track_caller]
    pub fn new(value: T) -> Self {
        let store = StoreSubscriptions::new();
        let value = CopyValue::new_maybe_sync(value);
        let value = value.into();

        let path = TinyVec::new();
        let selector = SelectorScope::new(path, store, value);
        selector.into()
    }
}

impl<T: ?Sized, Lens> Store<T, Lens> {
    /// Get the underlying selector for this store. The selector provides low level access to the lazy tracking system
    /// of the store. This can be useful to create selectors for custom data structures in libraries. For most applications
    /// the selectors generated by the [`Store`](dioxus_stores_macro::Store) macro provide all the functionality you need.
    pub fn selector(&self) -> &SelectorScope<Lens> {
        &self.selector
    }

    /// Convert the store into the underlying selector
    pub fn into_selector(self) -> SelectorScope<Lens> {
        self.selector
    }
}

impl<T: ?Sized, Lens> From<SelectorScope<Lens>> for Store<T, Lens> {
    fn from(selector: SelectorScope<Lens>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<T: ?Sized, Lens> PartialEq for Store<T, Lens>
where
    Lens: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}
impl<T: ?Sized, Lens> Clone for Store<T, Lens>
where
    Lens: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<T: ?Sized, Lens> Copy for Store<T, Lens> where Lens: Copy {}

impl<__F, __FMut, T: ?Sized, S, Lens> ::std::convert::From<MappedStore<T, Lens, __F, __FMut>>
    for WriteStore<T, S>
where
    Lens: Writable<Storage = S> + 'static,
    __F: Fn(&Lens::Target) -> &T + 'static,
    __FMut: Fn(&mut Lens::Target) -> &mut T + 'static,
    S: BoxedSignalStorage<T> + CreateBoxedSignalStorage<MappedMutSignal<T, Lens, __F, __FMut>>,
    T: 'static,
{
    fn from(value: MappedStore<T, Lens, __F, __FMut>) -> Self {
        Store {
            selector: value.selector.map_writer(::std::convert::Into::into),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<__F, __FMut, T: ?Sized, S, Lens> ::std::convert::From<MappedStore<T, Lens, __F, __FMut>>
    for ReadStore<T, S>
where
    Lens: Writable<Storage = S> + 'static,
    __F: Fn(&Lens::Target) -> &T + 'static,
    __FMut: Fn(&mut Lens::Target) -> &mut T + 'static,
    S: BoxedSignalStorage<T> + CreateBoxedSignalStorage<MappedMutSignal<T, Lens, __F, __FMut>>,
    T: 'static,
{
    fn from(value: MappedStore<T, Lens, __F, __FMut>) -> Self {
        Store {
            selector: value.selector.map_writer(::std::convert::Into::into),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<T, S> ::std::convert::From<WriteStore<T, S>> for ReadStore<T, S>
where
    T: ?Sized + 'static,
    S: BoxedSignalStorage<T> + CreateBoxedSignalStorage<WriteSignal<T, S>>,
{
    fn from(value: Store<T, WriteSignal<T, S>>) -> Self {
        Self {
            selector: value.selector.map_writer(::std::convert::Into::into),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<T, S> ::std::convert::From<Store<T, CopyValue<T, S>>> for ReadStore<T, S>
where
    T: 'static,
    S: BoxedSignalStorage<T> + CreateBoxedSignalStorage<CopyValue<T, S>> + Storage<T>,
{
    fn from(value: Store<T, CopyValue<T, S>>) -> Self {
        Self {
            selector: value.selector.map_writer(::std::convert::Into::into),
            _phantom: ::std::marker::PhantomData,
        }
    }
}
impl<T, S> ::std::convert::From<Store<T, CopyValue<T, S>>> for WriteStore<T, S>
where
    T: 'static,
    S: BoxedSignalStorage<T> + CreateBoxedSignalStorage<CopyValue<T, S>> + Storage<T>,
{
    fn from(value: Store<T, CopyValue<T, S>>) -> Self {
        Self {
            selector: value.selector.map_writer(::std::convert::Into::into),
            _phantom: ::std::marker::PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct SuperIntoReadSignalMarker;
impl<T, S, Lens> SuperInto<ReadSignal<T, S>, SuperIntoReadSignalMarker> for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Readable<Target = T, Storage = S> + 'static,
    S: CreateBoxedSignalStorage<Store<T, Lens>> + BoxedSignalStorage<T>,
{
    fn super_into(self) -> ReadSignal<T, S> {
        ReadSignal::new_maybe_sync(self)
    }
}

#[doc(hidden)]
pub struct SuperIntoWriteSignalMarker;
impl<T, S, Lens> SuperInto<WriteSignal<T, S>, SuperIntoWriteSignalMarker> for Store<T, Lens>
where
    T: ?Sized + 'static,
    Lens: Writable<Target = T, Storage = S> + 'static,
    S: CreateBoxedSignalStorage<Store<T, Lens>> + BoxedSignalStorage<T>,
{
    fn super_into(self) -> WriteSignal<T, S> {
        WriteSignal::new_maybe_sync(self)
    }
}

impl<T: ?Sized, Lens> Readable for Store<T, Lens>
where
    Lens: Readable<Target = T>,
    T: 'static,
{
    type Storage = Lens::Storage;
    type Target = T;
    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_read_unchecked()
    }
    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_peek_unchecked()
    }
    fn subscribers(&self) -> Subscribers {
        self.selector.subscribers()
    }
}
impl<T: ?Sized, Lens> Writable for Store<T, Lens>
where
    Lens: Writable<Target = T>,
    T: 'static,
{
    type WriteMetadata = Lens::WriteMetadata;
    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}
impl<T, Lens> IntoAttributeValue for Store<T, Lens>
where
    Self: Readable<Target = T>,
    T: ::std::clone::Clone + IntoAttributeValue + 'static,
{
    fn into_value(self) -> AttributeValue {
        ReadableExt::cloned(&self).into_value()
    }
}
impl<T, Lens> IntoDynNode for Store<T, Lens>
where
    Self: Readable<Target = T>,
    T: ::std::clone::Clone + IntoDynNode + 'static,
{
    fn into_dyn_node(self) -> DynamicNode {
        ReadableExt::cloned(&self).into_dyn_node()
    }
}
impl<T, Lens> ::std::ops::Deref for Store<T, Lens>
where
    Self: Readable<Target = T> + 'static,
    T: ::std::clone::Clone + 'static,
{
    type Target = dyn Fn() -> T;
    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

read_impls!(Store<T, Lens> where Lens: Readable<Target = T>);
write_impls!(Store<T, Lens> where Lens: Writable<Target = T>);

/// Create a new [`Store`]. Stores are a reactive type built for nested data structures.
///
///
/// By default stores act a lot like [`dioxus_signals::Signal`]s, but they provide more granular
/// subscriptions without requiring nested signals. You should derive [`Store`](dioxus_stores_macro::Store) on your data
/// structures to generate selectors that let you scope the store to a specific part of your data structure.
///
/// # Example
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// fn main() {
///     dioxus::launch(app);
/// }
///
/// // Deriving the store trait provides methods to scope the store to specific parts of your data structure.
/// // The `Store` macro generates a `count` and `children` method for `Store<CounterTree>`.
/// #[derive(Store, Default)]
/// struct CounterTree {
///     count: i32,
///     children: Vec<CounterTree>,
/// }
///
/// fn app() -> Element {
///     let value = use_store(Default::default);
///
///     rsx! {
///         Tree {
///             value
///         }
///     }
/// }
///
/// #[component]
/// fn Tree(value: Store<CounterTree>) -> Element {
///     // Calling the generated `count` method returns a new store that can only
///     // read and write the count field
///     let mut count = value.count();
///     let mut children = value.children();
///     rsx! {
///         button {
///             // Incrementing the count will only rerun parts of the app that have read the count field
///             onclick: move |_| count += 1,
///             "Increment"
///         }
///         button {
///             // Stores are aware of data structures like `Vec` and `Hashmap`. When we push an item to the vec
///             // it will only rerun the parts of the app that depend on the length of the vec
///             onclick: move |_| children.push(Default::default()),
///             "Push child"
///         }
///         ul {
///             // Iterating over the children gives us stores scoped to each child.
///             for value in children.iter() {
///                 li {
///                     Tree { value }
///                 }
///             }
///         }
///     }
/// }
/// ```
pub fn use_store<T: 'static>(init: impl FnOnce() -> T) -> Store<T> {
    use_hook(move || Store::new(init()))
}

/// Create a new [`SyncStore`]. Stores are a reactive type built for nested data structures.
/// `SyncStore` is a Store backed by `SyncStorage`.
///
/// Like [`use_store`], but produces `SyncStore<T>` instead of `Store<T>`
pub fn use_store_sync<T: Send + Sync + 'static>(init: impl FnOnce() -> T) -> SyncStore<T> {
    use_hook(|| Store::new_maybe_sync(init()))
}

/// A type alias for global stores
///
/// # Example
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// #[derive(Store)]
/// struct Counter {
///    count: i32,
/// }
///
/// static COUNTER: GlobalStore<Counter> = Global::new(|| Counter { count: 0 });
///
/// fn app() -> Element {
///     let mut count = COUNTER.resolve().count();
///
///     rsx! {
///         button {
///             onclick: move |_| count += 1,
///             "{count}"
///         }
///     }
/// }
/// ```
pub type GlobalStore<T> = Global<Store<T>, T>;

impl<T: 'static> InitializeFromFunction<T> for Store<T> {
    fn initialize_from_function(f: fn() -> T) -> Self {
        Store::new(f())
    }
}
