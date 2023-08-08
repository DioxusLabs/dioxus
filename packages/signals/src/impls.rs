use crate::rt::CopyValue;
use crate::signal::{ReadOnlySignal, Signal, Write};

use std::cell::{Ref, RefMut};

use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Sub},
};

macro_rules! read_impls {
    ($ty:ident) => {
        impl<T: Default + 'static> Default for $ty<T> {
            fn default() -> Self {
                Self::new(Default::default())
            }
        }

        impl<T> std::clone::Clone for $ty<T> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T> Copy for $ty<T> {}

        impl<T: Display + 'static> Display for $ty<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.with(|v| Display::fmt(v, f))
            }
        }

        impl<T: Debug + 'static> Debug for $ty<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.with(|v| Debug::fmt(v, f))
            }
        }

        impl<T: 'static> $ty<Vec<T>> {
            /// Read a value from the inner vector.
            pub fn get(&self, index: usize) -> Option<Ref<'_, T>> {
                Ref::filter_map(self.read(), |v| v.get(index)).ok()
            }
        }

        impl<T: 'static> $ty<Option<T>> {
            /// Unwraps the inner value and clones it.
            pub fn unwrap(&self) -> T
            where
                T: Clone,
            {
                self.with(|v| v.clone()).unwrap()
            }

            /// Attemps to read the inner value of the Option.
            pub fn as_ref(&self) -> Option<Ref<'_, T>> {
                Ref::filter_map(self.read(), |v| v.as_ref()).ok()
            }
        }
    };
}

macro_rules! write_impls {
    ($ty:ident) => {
        impl<T: Add<Output = T> + Copy + 'static> std::ops::Add<T> for $ty<T> {
            type Output = T;

            fn add(self, rhs: T) -> Self::Output {
                self.with(|v| *v + rhs)
            }
        }

        impl<T: Add<Output = T> + Copy + 'static> std::ops::AddAssign<T> for $ty<T> {
            fn add_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v + rhs)
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static> std::ops::SubAssign<T> for $ty<T> {
            fn sub_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v - rhs)
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static> std::ops::Sub<T> for $ty<T> {
            type Output = T;

            fn sub(self, rhs: T) -> Self::Output {
                self.with(|v| *v - rhs)
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static> std::ops::MulAssign<T> for $ty<T> {
            fn mul_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v * rhs)
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static> std::ops::Mul<T> for $ty<T> {
            type Output = T;

            fn mul(self, rhs: T) -> Self::Output {
                self.with(|v| *v * rhs)
            }
        }

        impl<T: Div<Output = T> + Copy + 'static> std::ops::DivAssign<T> for $ty<T> {
            fn div_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v / rhs)
            }
        }

        impl<T: Div<Output = T> + Copy + 'static> std::ops::Div<T> for $ty<T> {
            type Output = T;

            fn div(self, rhs: T) -> Self::Output {
                self.with(|v| *v / rhs)
            }
        }

        impl<T: 'static> $ty<Vec<T>> {
            /// Pushes a new value to the end of the vector.
            pub fn push(&self, value: T) {
                self.with_mut(|v| v.push(value))
            }

            /// Pops the last value from the vector.
            pub fn pop(&self) -> Option<T> {
                self.with_mut(|v| v.pop())
            }

            /// Inserts a new value at the given index.
            pub fn insert(&self, index: usize, value: T) {
                self.with_mut(|v| v.insert(index, value))
            }

            /// Removes the value at the given index.
            pub fn remove(&self, index: usize) -> T {
                self.with_mut(|v| v.remove(index))
            }

            /// Clears the vector, removing all values.
            pub fn clear(&self) {
                self.with_mut(|v| v.clear())
            }

            /// Extends the vector with the given iterator.
            pub fn extend(&self, iter: impl IntoIterator<Item = T>) {
                self.with_mut(|v| v.extend(iter))
            }

            /// Truncates the vector to the given length.
            pub fn truncate(&self, len: usize) {
                self.with_mut(|v| v.truncate(len))
            }

            /// Swaps two values in the vector.
            pub fn swap_remove(&self, index: usize) -> T {
                self.with_mut(|v| v.swap_remove(index))
            }

            /// Retains only the values that match the given predicate.
            pub fn retain(&self, f: impl FnMut(&T) -> bool) {
                self.with_mut(|v| v.retain(f))
            }

            /// Splits the vector into two at the given index.
            pub fn split_off(&self, at: usize) -> Vec<T> {
                self.with_mut(|v| v.split_off(at))
            }
        }

        impl<T: 'static> $ty<Option<T>> {
            /// Takes the value out of the Option.
            pub fn take(&self) -> Option<T> {
                self.with_mut(|v| v.take())
            }

            /// Replace the value in the Option.
            pub fn replace(&self, value: T) -> Option<T> {
                self.with_mut(|v| v.replace(value))
            }

            /// Gets the value out of the Option, or inserts the given value if the Option is empty.
            pub fn get_or_insert(&self, default: T) -> Ref<'_, T> {
                self.get_or_insert_with(|| default)
            }

            /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
            pub fn get_or_insert_with(&self, default: impl FnOnce() -> T) -> Ref<'_, T> {
                let borrow = self.read();
                if borrow.is_none() {
                    drop(borrow);
                    self.with_mut(|v| *v = Some(default()));
                    Ref::map(self.read(), |v| v.as_ref().unwrap())
                } else {
                    Ref::map(borrow, |v| v.as_ref().unwrap())
                }
            }
        }
    };
}

read_impls!(CopyValue);
write_impls!(CopyValue);
read_impls!(Signal);
write_impls!(Signal);
read_impls!(ReadOnlySignal);

/// An iterator over the values of a `CopyValue<Vec<T>>`.
pub struct CopyValueIterator<T: 'static> {
    index: usize,
    value: CopyValue<Vec<T>>,
}

impl<T: Clone> Iterator for CopyValueIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index).map(|v| v.clone())
    }
}

impl<T: Clone + 'static> IntoIterator for CopyValue<Vec<T>> {
    type IntoIter = CopyValueIterator<T>;

    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        CopyValueIterator {
            index: 0,
            value: self,
        }
    }
}

impl<T: 'static> CopyValue<Vec<T>> {
    /// Write to an element in the inner vector.
    pub fn get_mut(&self, index: usize) -> Option<RefMut<'_, T>> {
        RefMut::filter_map(self.write(), |v| v.get_mut(index)).ok()
    }
}

impl<T: 'static> CopyValue<Option<T>> {
    /// Deref the inner value mutably.
    pub fn as_mut(&self) -> Option<RefMut<'_, T>> {
        RefMut::filter_map(self.write(), |v| v.as_mut()).ok()
    }
}

/// An iterator over items in a `Signal<Vec<T>>`.
pub struct SignalIterator<T: 'static> {
    index: usize,
    value: Signal<Vec<T>>,
}

impl<T: Clone> Iterator for SignalIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index).map(|v| v.clone())
    }
}

impl<T: Clone + 'static> IntoIterator for Signal<Vec<T>> {
    type IntoIter = SignalIterator<T>;

    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        SignalIterator {
            index: 0,
            value: self,
        }
    }
}

impl<T: 'static> Signal<Vec<T>> {
    /// Returns a reference to an element or `None` if out of bounds.
    pub fn get_mut(&self, index: usize) -> Option<Write<'_, T, Vec<T>>> {
        Write::filter_map(self.write(), |v| v.get_mut(index))
    }
}

impl<T: 'static> Signal<Option<T>> {
    /// Returns a reference to an element or `None` if out of bounds.
    pub fn as_mut(&self) -> Option<Write<'_, T, Option<T>>> {
        Write::filter_map(self.write(), |v| v.as_mut())
    }
}
