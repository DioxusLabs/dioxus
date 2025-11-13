use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut, IndexMut},
};

use generational_box::{AnyStorage, UnsyncStorage};

use crate::{ext_methods, read::Readable, read::ReadableExt, MappedMutSignal, WriteSignal};

/// A reference to a value that can be written to.
#[allow(type_alias_bounds)]
pub type WritableRef<'a, T: Writable, O = <T as Readable>::Target> =
    WriteLock<'a, O, <T as Readable>::Storage, <T as Writable>::WriteMetadata>;

/// A trait for states that can be written to like [`crate::Signal`]. You may choose to accept this trait as a parameter instead of the concrete type to allow for more flexibility in your API.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// enum MyEnum {
///     String(String),
///     Number(i32),
/// }
///
/// fn MyComponent(mut count: Signal<MyEnum>) -> Element {
///     rsx! {
///         button {
///             onclick: move |_| {
///                 // You can use any methods from the Writable trait on Signals
///                 match &mut *count.write() {
///                     MyEnum::String(s) => s.push('a'),
///                     MyEnum::Number(n) => *n += 1,
///                 }
///             },
///             "Add value"
///         }
///     }
/// }
/// ```
pub trait Writable: Readable {
    /// Additional data associated with the write reference.
    type WriteMetadata;

    /// Try to get a mutable reference to the value without checking the lifetime. This will update any subscribers.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError>
    where
        Self::Target: 'static;
}

/// A mutable reference to a writable value. This reference acts similarly to [`std::cell::RefMut`], but it has extra debug information
/// and integrates with the reactive system to automatically update dependents.
///
/// [`WriteLock`] implements [`DerefMut`] which means you can call methods on the inner value just like you would on a mutable reference
/// to the inner value. If you need to get the inner reference directly, you can call [`WriteLock::deref_mut`].
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// fn app() -> Element {
///     let mut value = use_signal(|| String::from("hello"));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let mut mutable_reference = value.write();
///
///                 // You call methods like `push_str` on the reference just like you would with the inner String
///                 mutable_reference.push_str("world");
///             },
///             "Click to add world to the string"
///         }
///         div { "{value}" }
///     }
/// }
/// ```
///
/// ## Matching on WriteLock
///
/// You need to get the inner mutable reference with [`WriteLock::deref_mut`] before you match the inner value. If you try to match
/// without calling [`WriteLock::deref_mut`], you will get an error like this:
///
/// ```compile_fail
/// # use dioxus::prelude::*;
/// #[derive(Debug)]
/// enum Colors {
///     Red(u32),
///     Green
/// }
/// fn app() -> Element {
///     let mut value = use_signal(|| Colors::Red(0));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let mut mutable_reference = value.write();
///
///                 match mutable_reference {
///                     // Since we are matching on the `Write` type instead of &mut Colors, we can't match on the enum directly
///                     Colors::Red(brightness) => *brightness += 1,
///                     Colors::Green => {}
///                 }
///             },
///             "Click to add brightness to the red color"
///         }
///         div { "{value:?}" }
///     }
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> src/main.rs:18:21
///    |
/// 16 |                 match mutable_reference {
///    |                       ----------------- this expression has type `dioxus::prelude::Write<'_, Colors>`
/// 17 |                     // Since we are matching on the `Write` t...
/// 18 |                     Colors::Red(brightness) => *brightness += 1,
///    |                     ^^^^^^^^^^^^^^^^^^^^^^^ expected `Write<'_, Colors>`, found `Colors`
///    |
///    = note: expected struct `dioxus::prelude::Write<'_, Colors, >`
///                found enum `Colors`
/// ```
///
/// Instead, you need to call deref mut on the reference to get the inner value **before** you match on it:
///
/// ```rust
/// use std::ops::DerefMut;
/// # use dioxus::prelude::*;
/// #[derive(Debug)]
/// enum Colors {
///     Red(u32),
///     Green
/// }
/// fn app() -> Element {
///     let mut value = use_signal(|| Colors::Red(0));
///
///     rsx! {
///         button {
///             onclick: move |_| {
///                 let mut mutable_reference = value.write();
///
///                 // DerefMut converts the `Write` into a `&mut Colors`
///                 match mutable_reference.deref_mut() {
///                     // Now we can match on the inner value
///                     Colors::Red(brightness) => *brightness += 1,
///                     Colors::Green => {}
///                 }
///             },
///             "Click to add brightness to the red color"
///         }
///         div { "{value:?}" }
///     }
/// }
/// ```
///
/// ## Generics
/// - T is the current type of the write
/// - S is the storage type of the signal. This type determines if the signal is local to the current thread, or it can be shared across threads.
/// - D is the additional data associated with the write reference. This is used by signals to track when the write is dropped
pub struct WriteLock<'a, T: ?Sized + 'a, S: AnyStorage = UnsyncStorage, D = ()> {
    write: S::Mut<'a, T>,
    data: D,
}

impl<'a, T: ?Sized, S: AnyStorage> WriteLock<'a, T, S> {
    /// Create a new write reference
    pub fn new(write: S::Mut<'a, T>) -> Self {
        Self { write, data: () }
    }
}

impl<'a, T: ?Sized, S: AnyStorage, D> WriteLock<'a, T, S, D> {
    /// Create a new write reference with additional data.
    pub fn new_with_metadata(write: S::Mut<'a, T>, data: D) -> Self {
        Self { write, data }
    }

    /// Get the inner value of the write reference.
    pub fn into_inner(self) -> S::Mut<'a, T> {
        self.write
    }

    /// Get the additional data associated with the write reference.
    pub fn data(&self) -> &D {
        &self.data
    }

    /// Split into the inner value and the additional data.
    pub fn into_parts(self) -> (S::Mut<'a, T>, D) {
        (self.write, self.data)
    }

    /// Map the metadata of the write reference to a new type.
    pub fn map_metadata<O>(self, f: impl FnOnce(D) -> O) -> WriteLock<'a, T, S, O> {
        WriteLock {
            write: self.write,
            data: f(self.data),
        }
    }

    /// Map the mutable reference to the signal's value to a new type.
    pub fn map<O: ?Sized>(
        myself: Self,
        f: impl FnOnce(&mut T) -> &mut O,
    ) -> WriteLock<'a, O, S, D> {
        let Self { write, data, .. } = myself;
        WriteLock {
            write: S::map_mut(write, f),
            data,
        }
    }

    /// Try to map the mutable reference to the signal's value to a new type
    pub fn filter_map<O: ?Sized>(
        myself: Self,
        f: impl FnOnce(&mut T) -> Option<&mut O>,
    ) -> Option<WriteLock<'a, O, S, D>> {
        let Self { write, data, .. } = myself;
        let write = S::try_map_mut(write, f);
        write.map(|write| WriteLock { write, data })
    }

    /// Downcast the lifetime of the mutable reference to the signal's value.
    ///
    /// This function enforces the variance of the lifetime parameter `'a` in Mut.  Rust will typically infer this cast with a concrete type, but it cannot with a generic type.
    pub fn downcast_lifetime<'b>(mut_: Self) -> WriteLock<'b, T, S, D>
    where
        'a: 'b,
    {
        WriteLock {
            write: S::downcast_lifetime_mut(mut_.write),
            data: mut_.data,
        }
    }
}

impl<T, S, D> Deref for WriteLock<'_, T, S, D>
where
    S: AnyStorage,
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.write
    }
}

impl<T, S, D> DerefMut for WriteLock<'_, T, S, D>
where
    S: AnyStorage,
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write
    }
}

/// An extension trait for [`Writable`] that provides some convenience methods.
pub trait WritableExt: Writable {
    /// Get a mutable reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn write(&mut self) -> WritableRef<'_, Self>
    where
        Self::Target: 'static,
    {
        self.try_write().unwrap()
    }

    /// Try to get a mutable reference to the value.
    #[track_caller]
    fn try_write(&mut self) -> Result<WritableRef<'_, Self>, generational_box::BorrowMutError>
    where
        Self::Target: 'static,
    {
        self.try_write_unchecked().map(WriteLock::downcast_lifetime)
    }

    /// Get a mutable reference to the value without checking the lifetime. This will update any subscribers.
    ///
    /// NOTE: This method is completely safe because borrow checking is done at runtime.
    #[track_caller]
    fn write_unchecked(&self) -> WritableRef<'static, Self>
    where
        Self::Target: 'static,
    {
        self.try_write_unchecked().unwrap()
    }

    /// Map the references and mutable references of the writable value to a new type. This lets you provide a view
    /// into the writable value without creating a new signal or cloning the value.
    ///
    /// Anything that subscribes to the writable value will be rerun whenever the original value changes or you write to this
    /// scoped value, even if the view does not change. If you want to memorize the view, you can use a [`crate::Memo`] instead.
    /// For fine grained scoped updates, use stores instead
    ///
    /// # Example
    /// ```rust
    /// # use dioxus::prelude::*;
    /// fn List(list: Signal<Vec<i32>>) -> Element {
    ///     rsx! {
    ///         for index in 0..list.len() {
    ///             // We can use the `map` method to provide a view into the single item in the list that the child component will render
    ///             Item { item: list.map_mut(move |v| &v[index], move |v| &mut v[index]) }
    ///         }
    ///     }
    /// }
    ///
    /// // The child component doesn't need to know that the mapped value is coming from a list
    /// #[component]
    /// fn Item(item: WriteSignal<i32>) -> Element {
    ///     rsx! {
    ///         button {
    ///             onclick: move |_| *item.write() += 1,
    ///             "{item}"
    ///         }
    ///     }
    /// }
    /// ```
    fn map_mut<O, F, FMut>(self, f: F, f_mut: FMut) -> MappedMutSignal<O, Self, F, FMut>
    where
        Self: Sized,
        O: ?Sized,
        F: Fn(&Self::Target) -> &O,
        FMut: Fn(&mut Self::Target) -> &mut O,
    {
        MappedMutSignal::new(self, f, f_mut)
    }

    /// Run a function with a mutable reference to the value. If the value has been dropped, this will panic.
    #[track_caller]
    fn with_mut<O>(&mut self, f: impl FnOnce(&mut Self::Target) -> O) -> O
    where
        Self::Target: 'static,
    {
        f(&mut *self.write())
    }

    /// Set the value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    fn set(&mut self, value: Self::Target)
    where
        Self::Target: Sized + 'static,
    {
        *self.write() = value;
    }

    /// Invert the boolean value of the signal. This will trigger an update on all subscribers.
    #[track_caller]
    fn toggle(&mut self)
    where
        Self::Target: std::ops::Not<Output = Self::Target> + Clone + 'static,
    {
        let inverted = !(*self.peek()).clone();
        self.set(inverted);
    }

    /// Index into the inner value and return a reference to the result.
    #[track_caller]
    fn index_mut<I>(
        &mut self,
        index: I,
    ) -> WritableRef<'_, Self, <Self::Target as std::ops::Index<I>>::Output>
    where
        Self::Target: std::ops::IndexMut<I> + 'static,
    {
        WriteLock::map(self.write(), |v| v.index_mut(index))
    }

    /// Takes the value out of the Signal, leaving a Default in its place.
    #[track_caller]
    fn take(&mut self) -> Self::Target
    where
        Self::Target: Default + 'static,
    {
        self.with_mut(std::mem::take)
    }

    /// Replace the value in the Signal, returning the old value.
    #[track_caller]
    fn replace(&mut self, value: Self::Target) -> Self::Target
    where
        Self::Target: Sized + 'static,
    {
        self.with_mut(|v| std::mem::replace(v, value))
    }
}

impl<W: Writable + ?Sized> WritableExt for W {}

/// An extension trait for [`Writable`] values that can be boxed into a trait object.
pub trait WritableBoxedExt: Writable<Storage = UnsyncStorage> {
    /// Box the writable value into a trait object. This is useful for passing around writable values without knowing their concrete type.
    fn boxed_mut(self) -> WriteSignal<Self::Target>
    where
        Self: Sized + 'static,
    {
        WriteSignal::new(self)
    }
}

impl<T: Writable<Storage = UnsyncStorage> + 'static> WritableBoxedExt for T {
    fn boxed_mut(self) -> WriteSignal<Self::Target> {
        WriteSignal::new(self)
    }
}

/// An extension trait for [`Writable<Option<T>>`]` that provides some convenience methods.
pub trait WritableOptionExt<T>: Writable<Target = Option<T>> {
    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    #[track_caller]
    fn get_or_insert(&mut self, default: T) -> WritableRef<'_, Self, T>
    where
        T: 'static,
    {
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    #[track_caller]
    fn get_or_insert_with(&mut self, default: impl FnOnce() -> T) -> WritableRef<'_, Self, T>
    where
        T: 'static,
    {
        let is_none = self.read().is_none();
        if is_none {
            self.with_mut(|v| *v = Some(default()));
            WriteLock::map(self.write(), |v| v.as_mut().unwrap())
        } else {
            WriteLock::map(self.write(), |v| v.as_mut().unwrap())
        }
    }

    /// Attempts to write the inner value of the Option.
    #[track_caller]
    fn as_mut(&mut self) -> Option<WritableRef<'_, Self, T>>
    where
        T: 'static,
    {
        WriteLock::filter_map(self.write(), |v: &mut Option<T>| v.as_mut())
    }
}

impl<T, W> WritableOptionExt<T> for W where W: Writable<Target = Option<T>> {}

/// An extension trait for [`Writable<Vec<T>>`] that provides some convenience methods.
pub trait WritableVecExt<T>: Writable<Target = Vec<T>> {
    /// Pushes a new value to the end of the vector.
    #[track_caller]
    fn push(&mut self, value: T)
    where
        T: 'static,
    {
        self.with_mut(|v| v.push(value))
    }

    /// Pops the last value from the vector.
    #[track_caller]
    fn pop(&mut self) -> Option<T>
    where
        T: 'static,
    {
        self.with_mut(|v| v.pop())
    }

    /// Inserts a new value at the given index.
    #[track_caller]
    fn insert(&mut self, index: usize, value: T)
    where
        T: 'static,
    {
        self.with_mut(|v| v.insert(index, value))
    }

    /// Removes the value at the given index.
    #[track_caller]
    fn remove(&mut self, index: usize) -> T
    where
        T: 'static,
    {
        self.with_mut(|v| v.remove(index))
    }

    /// Clears the vector, removing all values.
    #[track_caller]
    fn clear(&mut self)
    where
        T: 'static,
    {
        self.with_mut(|v| v.clear())
    }

    /// Extends the vector with the given iterator.
    #[track_caller]
    fn extend(&mut self, iter: impl IntoIterator<Item = T>)
    where
        T: 'static,
    {
        self.with_mut(|v| v.extend(iter))
    }

    /// Truncates the vector to the given length.
    #[track_caller]
    fn truncate(&mut self, len: usize)
    where
        T: 'static,
    {
        self.with_mut(|v| v.truncate(len))
    }

    /// Swaps two values in the vector.
    #[track_caller]
    fn swap_remove(&mut self, index: usize) -> T
    where
        T: 'static,
    {
        self.with_mut(|v| v.swap_remove(index))
    }

    /// Retains only the values that match the given predicate.
    #[track_caller]
    fn retain(&mut self, f: impl FnMut(&T) -> bool)
    where
        T: 'static,
    {
        self.with_mut(|v| v.retain(f))
    }

    /// Splits the vector into two at the given index.
    #[track_caller]
    fn split_off(&mut self, at: usize) -> Vec<T>
    where
        T: 'static,
    {
        self.with_mut(|v| v.split_off(at))
    }

    /// Try to mutably get an element from the vector.
    #[track_caller]
    fn get_mut(&mut self, index: usize) -> Option<WritableRef<'_, Self, T>>
    where
        T: 'static,
    {
        WriteLock::filter_map(self.write(), |v: &mut Vec<T>| v.get_mut(index))
    }

    /// Gets an iterator over the values of the vector.
    #[track_caller]
    fn iter_mut(&mut self) -> WritableValueIterator<'_, Self>
    where
        Self: Sized + Clone,
    {
        WritableValueIterator {
            index: 0,
            value: self,
        }
    }
}

/// An iterator over the values of a [`Writable<Vec<T>>`].
pub struct WritableValueIterator<'a, R> {
    index: usize,
    value: &'a mut R,
}

impl<'a, T: 'static, R: Writable<Target = Vec<T>>> Iterator for WritableValueIterator<'a, R> {
    type Item = WritableRef<'a, R, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        WriteLock::filter_map(
            self.value.try_write_unchecked().unwrap(),
            |v: &mut Vec<T>| v.get_mut(index),
        )
        .map(WriteLock::downcast_lifetime)
    }
}

impl<W, T> WritableVecExt<T> for W where W: Writable<Target = Vec<T>> {}

/// An extension trait for [`Writable<String>`] that provides some convenience methods.
pub trait WritableStringExt: Writable<Target = String> {
    ext_methods! {
        /// Pushes a character to the end of the string.
        fn push_str(&mut self, s: &str) = String::push_str;

        /// Pushes a character to the end of the string.
        fn push(&mut self, c: char) = String::push;

        /// Pops a character from the end of the string.
        fn pop(&mut self) -> Option<char> = String::pop;

        /// Inserts a string at the given index.
        fn insert_str(&mut self, idx: usize, s: &str) = String::insert_str;

        /// Inserts a character at the given index.
        fn insert(&mut self, idx: usize, c: char) = String::insert;

        /// Remove a character at the given index
        fn remove(&mut self, idx: usize) -> char = String::remove;

        /// Replace a range of the string with the given string.
        fn replace_range(&mut self, range: impl std::ops::RangeBounds<usize>, replace_with: &str) = String::replace_range;

        /// Clears the string, removing all characters.
        fn clear(&mut self) = String::clear;

        /// Extends the string with the given iterator of characters.
        fn extend(&mut self, iter: impl IntoIterator<Item = char>) = String::extend;

        /// Truncates the string to the given length.
        fn truncate(&mut self, len: usize) = String::truncate;

        /// Splits the string off at the given index, returning the tail as a new string.
        fn split_off(&mut self, at: usize) -> String = String::split_off;
    }
}

impl<W> WritableStringExt for W where W: Writable<Target = String> {}

/// An extension trait for [`Writable<HashMap<K, V, H>>`] that provides some convenience methods.
pub trait WritableHashMapExt<K: 'static, V: 'static, H: 'static>:
    Writable<Target = HashMap<K, V, H>>
{
    ext_methods! {
        /// Clears the map, removing all key-value pairs.
        fn clear(&mut self) = HashMap::clear;

        /// Retains only the key-value pairs that match the given predicate.
        fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) = HashMap::retain;
    }

    /// Inserts a key-value pair into the map. If the key was already present, the old value is returned.
    #[track_caller]
    fn insert(&mut self, k: K, v: V) -> Option<V>
    where
        K: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        self.with_mut(|map: &mut HashMap<K, V, H>| map.insert(k, v))
    }

    /// Extends the map with the key-value pairs from the given iterator.
    #[track_caller]
    fn extend(&mut self, iter: impl IntoIterator<Item = (K, V)>)
    where
        K: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        self.with_mut(|map: &mut HashMap<K, V, H>| map.extend(iter))
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    #[track_caller]
    fn remove(&mut self, k: &K) -> Option<V>
    where
        K: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        self.with_mut(|map: &mut HashMap<K, V, H>| map.remove(k))
    }

    /// Get a mutable reference to the value at the given key.
    #[track_caller]
    fn get_mut(&mut self, k: &K) -> Option<WritableRef<'_, Self, V>>
    where
        K: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        WriteLock::filter_map(self.write(), |map: &mut HashMap<K, V, H>| map.get_mut(k))
    }
}

impl<K: 'static, V: 'static, H: 'static, R> WritableHashMapExt<K, V, H> for R where
    R: Writable<Target = HashMap<K, V, H>>
{
}

/// An extension trait for [`Writable<HashSet<V, H>>`] that provides some convenience methods.
pub trait WritableHashSetExt<V: 'static, H: 'static>: Writable<Target = HashSet<V, H>> {
    ext_methods! {
        /// Clear the hash set.
        fn clear(&mut self) = HashSet::clear;

        /// Retain only the elements specified by the predicate.
        fn retain(&mut self, f: impl FnMut(&V) -> bool) = HashSet::retain;
    }

    /// Inserts a value into the set. Returns true if the value was not already present.
    #[track_caller]
    fn insert(&mut self, k: V) -> bool
    where
        V: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        self.with_mut(|set| set.insert(k))
    }

    /// Extends the set with the values from the given iterator.
    #[track_caller]
    fn extend(&mut self, iter: impl IntoIterator<Item = V>)
    where
        V: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        self.with_mut(|set| set.extend(iter))
    }

    /// Removes a value from the set. Returns true if the value was present.
    #[track_caller]
    fn remove(&mut self, k: &V) -> bool
    where
        V: std::cmp::Eq + std::hash::Hash,
        H: std::hash::BuildHasher,
    {
        self.with_mut(|set| set.remove(k))
    }
}

impl<V: 'static, H: 'static, R> WritableHashSetExt<V, H> for R where
    R: Writable<Target = HashSet<V, H>>
{
}
