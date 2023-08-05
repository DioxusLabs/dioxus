use crate::rt::CopyValue;
use crate::Signal;

use std::cell::{Ref, RefMut};

use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Sub},
};

macro_rules! impls {
    ($ty:ident) => {
        impl<T: Default + 'static> Default for $ty<T> {
            fn default() -> Self {
                Self::new(T::default())
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

        impl<T: Add<Output = T> + Copy + 'static> std::ops::AddAssign<T> for $ty<T> {
            fn add_assign(&mut self, rhs: T) {
                self.set(self.value() + rhs);
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static> std::ops::SubAssign<T> for $ty<T> {
            fn sub_assign(&mut self, rhs: T) {
                self.set(self.value() - rhs);
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static> std::ops::MulAssign<T> for $ty<T> {
            fn mul_assign(&mut self, rhs: T) {
                self.set(self.value() * rhs);
            }
        }

        impl<T: Div<Output = T> + Copy + 'static> std::ops::DivAssign<T> for $ty<T> {
            fn div_assign(&mut self, rhs: T) {
                self.set(self.value() / rhs);
            }
        }

        impl<T: 'static> $ty<Vec<T>> {
            pub fn push(&self, value: T) {
                self.with_mut(|v| v.push(value))
            }

            pub fn pop(&self) -> Option<T> {
                self.with_mut(|v| v.pop())
            }

            pub fn insert(&self, index: usize, value: T) {
                self.with_mut(|v| v.insert(index, value))
            }

            pub fn remove(&self, index: usize) -> T {
                self.with_mut(|v| v.remove(index))
            }

            pub fn clear(&self) {
                self.with_mut(|v| v.clear())
            }

            pub fn extend(&self, iter: impl IntoIterator<Item = T>) {
                self.with_mut(|v| v.extend(iter))
            }

            pub fn truncate(&self, len: usize) {
                self.with_mut(|v| v.truncate(len))
            }

            pub fn swap_remove(&self, index: usize) -> T {
                self.with_mut(|v| v.swap_remove(index))
            }

            pub fn retain(&self, f: impl FnMut(&T) -> bool) {
                self.with_mut(|v| v.retain(f))
            }

            pub fn split_off(&self, at: usize) -> Vec<T> {
                self.with_mut(|v| v.split_off(at))
            }

            pub fn get(&self, index: usize) -> Option<Ref<'_, T>> {
                Ref::filter_map(self.read(), |v| v.get(index)).ok()
            }

            pub fn get_mut(&self, index: usize) -> Option<RefMut<'_, T>> {
                RefMut::filter_map(self.write(), |v| v.get_mut(index)).ok()
            }
        }

        impl<T: 'static> $ty<Option<T>> {
            pub fn take(&self) -> Option<T> {
                self.with_mut(|v| v.take())
            }

            pub fn replace(&self, value: T) -> Option<T> {
                self.with_mut(|v| v.replace(value))
            }

            pub fn unwrap(&self) -> T
            where
                T: Clone,
            {
                self.with(|v| v.clone()).unwrap()
            }

            pub fn as_ref(&self) -> Option<Ref<'_, T>> {
                Ref::filter_map(self.read(), |v| v.as_ref()).ok()
            }

            pub fn as_mut(&self) -> Option<RefMut<'_, T>> {
                RefMut::filter_map(self.write(), |v| v.as_mut()).ok()
            }

            pub fn get_or_insert(&self, default: T) -> Ref<'_, T> {
                self.get_or_insert_with(|| default)
            }

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

impls!(CopyValue);
impls!(Signal);

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

pub struct CopySignalIterator<T: 'static> {
    index: usize,
    value: Signal<Vec<T>>,
}

impl<T: Clone> Iterator for CopySignalIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index).map(|v| v.clone())
    }
}

impl<T: Clone + 'static> IntoIterator for Signal<Vec<T>> {
    type IntoIter = CopySignalIterator<T>;

    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        CopySignalIterator {
            index: 0,
            value: self,
        }
    }
}
