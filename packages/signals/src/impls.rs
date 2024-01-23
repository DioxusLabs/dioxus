use crate::read::Readable;
use crate::rt::CopyValue;
use crate::signal::{Signal, Write};
use crate::write::Writable;
use crate::{GlobalMemo, GlobalSignal, ReadOnlySignal, SignalData};
use generational_box::{AnyStorage, Storage};
use generational_box::{GenerationalRef, UnsyncStorage};

use std::cell::Ref;
use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Sub},
};

macro_rules! read_impls {
    ($ty:ident $(: $extra_bounds:path)? $(, $bound_ty:ident : $bound:path, $vec_bound_ty:ident : $vec_bound:path)?) => {
        $(
            impl<T: Default + 'static, $bound_ty: $bound> Default for $ty<T, $bound_ty> {
                #[track_caller]
                fn default() -> Self {
                    Self::new_maybe_sync(Default::default())
                }
            }
        )?

        impl<T $(: $extra_bounds)? $(,$bound_ty: $bound)?> std::clone::Clone for $ty<T $(, $bound_ty)?> {
            #[track_caller]
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T $(: $extra_bounds)? $(,$bound_ty: $bound)?> Copy for $ty<T $(, $bound_ty)?> {}

        impl<T: $($extra_bounds + )? Display + 'static $(,$bound_ty: $bound)?> Display for $ty<T $(, $bound_ty)?> {
            #[track_caller]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.with(|v| Display::fmt(v, f))
            }
        }

        impl<T: $($extra_bounds + )? Debug + 'static $(,$bound_ty: $bound)?> Debug for $ty<T $(, $bound_ty)?> {
            #[track_caller]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.with(|v| Debug::fmt(v, f))
            }
        }

        impl<T: $($extra_bounds + )? PartialEq + 'static $(,$bound_ty: $bound)?> PartialEq<T> for $ty<T $(, $bound_ty)?> {
            #[track_caller]
            fn eq(&self, other: &T) -> bool {
                self.with(|v| *v == *other)
            }
        }

        impl<T: $($extra_bounds + )? 'static $(,$vec_bound_ty: $vec_bound)?> $ty<Vec<T>, $($vec_bound_ty)?> {
            /// Returns the length of the inner vector.
            #[track_caller]
            pub fn len(&self) -> usize {
                self.with(|v| v.len())
            }

            /// Returns true if the inner vector is empty.
            #[track_caller]
            pub fn is_empty(&self) -> bool {
                self.with(|v| v.is_empty())
            }
        }
    };
}

macro_rules! write_impls {
    ($ty:ident, $bound:path, $vec_bound:path) => {
        impl<T: Add<Output = T> + Copy + 'static, S: $bound> std::ops::Add<T> for $ty<T, S> {
            type Output = T;

            #[track_caller]
            fn add(self, rhs: T) -> Self::Output {
                self.with(|v| *v + rhs)
            }
        }

        impl<T: Add<Output = T> + Copy + 'static, S: $bound> std::ops::AddAssign<T> for $ty<T, S> {
            #[track_caller]
            fn add_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v + rhs)
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static, S: $bound> std::ops::SubAssign<T> for $ty<T, S> {
            #[track_caller]
            fn sub_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v - rhs)
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static, S: $bound> std::ops::Sub<T> for $ty<T, S> {
            type Output = T;

            #[track_caller]
            fn sub(self, rhs: T) -> Self::Output {
                self.with(|v| *v - rhs)
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static, S: $bound> std::ops::MulAssign<T> for $ty<T, S> {
            #[track_caller]
            fn mul_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v * rhs)
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static, S: $bound> std::ops::Mul<T> for $ty<T, S> {
            type Output = T;

            #[track_caller]
            fn mul(self, rhs: T) -> Self::Output {
                self.with(|v| *v * rhs)
            }
        }

        impl<T: Div<Output = T> + Copy + 'static, S: $bound> std::ops::DivAssign<T> for $ty<T, S> {
            #[track_caller]
            fn div_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v / rhs)
            }
        }

        impl<T: Div<Output = T> + Copy + 'static, S: $bound> std::ops::Div<T> for $ty<T, S> {
            type Output = T;

            #[track_caller]
            fn div(self, rhs: T) -> Self::Output {
                self.with(|v| *v / rhs)
            }
        }

        write_vec_impls!($ty, S: $vec_bound);
    };
}

macro_rules! write_vec_impls {
    ($ty:ident $(, $vec_bound_ty:ident: $vec_bound:path)?) => {
        impl<T: 'static $(, $vec_bound_ty: $vec_bound)?> $ty<Vec<T> $(, $vec_bound_ty)?> {
            /// Pushes a new value to the end of the vector.
            #[track_caller]
            pub fn push(&mut self, value: T) {
                self.with_mut(|v| v.push(value))
            }

            /// Pops the last value from the vector.
            #[track_caller]
            pub fn pop(&mut self) -> Option<T> {
                self.with_mut(|v| v.pop())
            }

            /// Inserts a new value at the given index.
            #[track_caller]
            pub fn insert(&mut self, index: usize, value: T) {
                self.with_mut(|v| v.insert(index, value))
            }

            /// Removes the value at the given index.
            #[track_caller]
            pub fn remove(&mut self, index: usize) -> T {
                self.with_mut(|v| v.remove(index))
            }

            /// Clears the vector, removing all values.
            #[track_caller]
            pub fn clear(&mut self) {
                self.with_mut(|v| v.clear())
            }

            /// Extends the vector with the given iterator.
            #[track_caller]
            pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
                self.with_mut(|v| v.extend(iter))
            }

            /// Truncates the vector to the given length.
            #[track_caller]
            pub fn truncate(&mut self, len: usize) {
                self.with_mut(|v| v.truncate(len))
            }

            /// Swaps two values in the vector.
            #[track_caller]
            pub fn swap_remove(&mut self, index: usize) -> T {
                self.with_mut(|v| v.swap_remove(index))
            }

            /// Retains only the values that match the given predicate.
            #[track_caller]
            pub fn retain(&mut self, f: impl FnMut(&T) -> bool) {
                self.with_mut(|v| v.retain(f))
            }

            /// Splits the vector into two at the given index.
            #[track_caller]
            pub fn split_off(&mut self, at: usize) -> Vec<T> {
                self.with_mut(|v| v.split_off(at))
            }
        }
    };
}

read_impls!(CopyValue, S: Storage<T>, S: Storage<Vec<T>>);

impl<T: 'static, S: Storage<Vec<T>>> CopyValue<Vec<T>, S> {
    /// Read a value from the inner vector.
    #[track_caller]
    pub fn get(&self, index: usize) -> Option<S::Ref<T>> {
        S::try_map(self.read(), move |v| v.get(index))
    }
}

impl<T: 'static, S: Storage<Option<T>>> CopyValue<Option<T>, S> {
    /// Unwraps the inner value and clones it.
    #[track_caller]
    pub fn unwrap(&self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attempts to read the inner value of the Option.
    #[track_caller]
    pub fn as_ref(&self) -> Option<S::Ref<T>> {
        S::try_map(self.read(), |v| v.as_ref())
    }
}

write_impls!(CopyValue, Storage<T>, Storage<Vec<T>>);

impl<T: 'static, S: Storage<Option<T>>> CopyValue<Option<T>, S> {
    /// Takes the value out of the Option.
    #[track_caller]
    pub fn take(&self) -> Option<T> {
        self.with_mut(|v| v.take())
    }

    /// Replace the value in the Option.
    #[track_caller]
    pub fn replace(&self, value: T) -> Option<T> {
        self.with_mut(|v| v.replace(value))
    }

    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    #[track_caller]
    pub fn get_or_insert(&self, default: T) -> S::Ref<T> {
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    #[track_caller]
    pub fn get_or_insert_with(&self, default: impl FnOnce() -> T) -> S::Ref<T> {
        let borrow = self.read();
        if borrow.is_none() {
            drop(borrow);
            self.with_mut(|v| *v = Some(default()));
            S::map(self.read(), |v| v.as_ref().unwrap())
        } else {
            S::map(borrow, |v| v.as_ref().unwrap())
        }
    }
}

read_impls!(Signal, S: Storage<SignalData<T>>, S: Storage<SignalData<Vec<T>>>);

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> Signal<Vec<T>, S> {
    /// Read a value from the inner vector.
    pub fn get(&self, index: usize) -> Option<S::Ref<T>> {
        S::try_map(self.read(), move |v| v.get(index))
    }
}

impl<T: 'static, S: Storage<SignalData<Option<T>>>> Signal<Option<T>, S> {
    /// Unwraps the inner value and clones it.
    pub fn unwrap(&self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attempts to read the inner value of the Option.
    pub fn as_ref(&self) -> Option<S::Ref<T>> {
        S::try_map(self.read(), |v| v.as_ref())
    }
}

write_impls!(Signal, Storage<SignalData<T>>, Storage<SignalData<Vec<T>>>);

impl<T: 'static, S: Storage<SignalData<Option<T>>>> Signal<Option<T>, S> {
    /// Takes the value out of the Option.
    pub fn take(&mut self) -> Option<T> {
        self.with_mut(|v| v.take())
    }

    /// Replace the value in the Option.
    pub fn replace(&mut self, value: T) -> Option<T> {
        self.with_mut(|v| v.replace(value))
    }

    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    pub fn get_or_insert(&mut self, default: T) -> S::Ref<T> {
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    pub fn get_or_insert_with(&mut self, default: impl FnOnce() -> T) -> S::Ref<T> {
        let borrow = self.read();
        if borrow.is_none() {
            drop(borrow);
            self.with_mut(|v| *v = Some(default()));
            S::map(self.read(), |v| v.as_ref().unwrap())
        } else {
            S::map(borrow, |v| v.as_ref().unwrap())
        }
    }
}

read_impls!(
    ReadOnlySignal,
    S: Storage<SignalData<T>>,
    S: Storage<SignalData<Vec<T>>>
);

read_impls!(GlobalSignal);

impl<T: 'static> GlobalSignal<Vec<T>> {
    /// Read a value from the inner vector.
    pub fn get(&'static self, index: usize) -> Option<GenerationalRef<Ref<'static, T>>> {
        <UnsyncStorage as AnyStorage>::try_map(self.read(), move |v| v.get(index))
    }
}

impl<T: 'static> GlobalSignal<Option<T>> {
    /// Unwraps the inner value and clones it.
    pub fn unwrap(&'static self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attempts to read the inner value of the Option.
    pub fn as_ref(&'static self) -> Option<GenerationalRef<Ref<'static, T>>> {
        <UnsyncStorage as AnyStorage>::try_map(self.read(), |v| v.as_ref())
    }
}

write_vec_impls!(GlobalSignal);

impl<T: 'static> GlobalSignal<Option<T>> {
    /// Takes the value out of the Option.
    pub fn take(&self) -> Option<T> {
        self.with_mut(|v| v.take())
    }

    /// Replace the value in the Option.
    pub fn replace(&self, value: T) -> Option<T> {
        self.with_mut(|v| v.replace(value))
    }

    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    pub fn get_or_insert(&self, default: T) -> GenerationalRef<Ref<'static, T>> {
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    pub fn get_or_insert_with(
        &self,
        default: impl FnOnce() -> T,
    ) -> GenerationalRef<Ref<'static, T>> {
        let borrow = self.read();
        if borrow.is_none() {
            drop(borrow);
            self.with_mut(|v| *v = Some(default()));
            <UnsyncStorage as AnyStorage>::map(self.read(), |v| v.as_ref().unwrap())
        } else {
            <UnsyncStorage as AnyStorage>::map(borrow, |v| v.as_ref().unwrap())
        }
    }
}

read_impls!(GlobalMemo: PartialEq);

impl<T: PartialEq + 'static> GlobalMemo<Vec<T>> {
    /// Read a value from the inner vector.
    pub fn get(&'static self, index: usize) -> Option<GenerationalRef<Ref<'static, T>>> {
        <UnsyncStorage as AnyStorage>::try_map(self.read(), move |v| v.get(index))
    }
}

impl<T: PartialEq + 'static> GlobalMemo<Option<T>> {
    /// Unwraps the inner value and clones it.
    pub fn unwrap(&'static self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attempts to read the inner value of the Option.
    pub fn as_ref(&'static self) -> Option<GenerationalRef<Ref<'static, T>>> {
        <UnsyncStorage as AnyStorage>::try_map(self.read(), |v| v.as_ref())
    }
}

/// An iterator over the values of a `CopyValue<Vec<T>>`.
pub struct CopyValueIterator<T: 'static, S: Storage<Vec<T>>> {
    index: usize,
    value: CopyValue<Vec<T>, S>,
}

impl<T, S: Storage<Vec<T>>> Iterator for CopyValueIterator<T, S> {
    type Item = S::Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index)
    }
}

impl<T: 'static, S: Storage<Vec<T>>> IntoIterator for CopyValue<Vec<T>, S> {
    type IntoIter = CopyValueIterator<T, S>;

    type Item = S::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        CopyValueIterator {
            index: 0,
            value: self,
        }
    }
}

impl<T: 'static, S: Storage<Vec<T>>> CopyValue<Vec<T>, S> {
    /// Write to an element in the inner vector.
    pub fn get_mut(&self, index: usize) -> Option<S::Mut<T>> {
        S::try_map_mut(self.write(), |v: &mut Vec<T>| v.get_mut(index))
    }
}

impl<T: 'static, S: Storage<Option<T>>> CopyValue<Option<T>, S> {
    /// Deref the inner value mutably.
    pub fn as_mut(&self) -> Option<S::Mut<T>> {
        S::try_map_mut(self.write(), |v: &mut Option<T>| v.as_mut())
    }
}

/// An iterator over items in a `Signal<Vec<T>>`.
pub struct SignalIterator<T: 'static, S: Storage<SignalData<Vec<T>>>> {
    index: usize,
    value: Signal<Vec<T>, S>,
}

impl<T, S: Storage<SignalData<Vec<T>>>> Iterator for SignalIterator<T, S> {
    type Item = S::Ref<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index)
    }
}

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> IntoIterator for Signal<Vec<T>, S> {
    type IntoIter = SignalIterator<T, S>;

    type Item = S::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        SignalIterator {
            index: 0,
            value: self,
        }
    }
}

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> Signal<Vec<T>, S> {
    /// Returns a reference to an element or `None` if out of bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<Write<T, S>> {
        Write::filter_map(self.write(), |v| v.get_mut(index))
    }
}

impl<T: 'static, S: Storage<SignalData<Option<T>>>> Signal<Option<T>, S> {
    /// Returns a reference to an element or `None` if out of bounds.
    pub fn as_mut(&mut self) -> Option<Write<T, S>> {
        Write::filter_map(self.write(), |v| v.as_mut())
    }
}
