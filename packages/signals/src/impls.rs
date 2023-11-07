use crate::rt::CopyValue;
use crate::signal::{ReadOnlySignal, Signal, Write};
use crate::SignalData;
use generational_box::Mappable;
use generational_box::{MappableMut, Storage};

use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Sub},
};

macro_rules! read_impls {
    ($ty:ident, $bound:path) => {
        impl<T: Default + 'static, S: $bound> Default for $ty<T, S> {
            fn default() -> Self {
                Self::new_maybe_sync(Default::default())
            }
        }

        impl<T, S: $bound> std::clone::Clone for $ty<T, S> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T, S: $bound + Copy> Copy for $ty<T, S> {}

        impl<T: Display + 'static, S: $bound> Display for $ty<T, S> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.with(|v| Display::fmt(v, f))
            }
        }

        impl<T: Debug + 'static, S: $bound> Debug for $ty<T, S> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.with(|v| Debug::fmt(v, f))
            }
        }
    };
}

macro_rules! write_impls {
    ($ty:ident, $bound:path, $vec_bound:path) => {
        impl<T: Add<Output = T> + Copy + 'static, S: $bound> std::ops::Add<T> for $ty<T, S> {
            type Output = T;

            fn add(self, rhs: T) -> Self::Output {
                self.with(|v| *v + rhs)
            }
        }

        impl<T: Add<Output = T> + Copy + 'static, S: $bound> std::ops::AddAssign<T> for $ty<T, S> {
            fn add_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v + rhs)
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static, S: $bound> std::ops::SubAssign<T> for $ty<T, S> {
            fn sub_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v - rhs)
            }
        }

        impl<T: Sub<Output = T> + Copy + 'static, S: $bound> std::ops::Sub<T> for $ty<T, S> {
            type Output = T;

            fn sub(self, rhs: T) -> Self::Output {
                self.with(|v| *v - rhs)
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static, S: $bound> std::ops::MulAssign<T> for $ty<T, S> {
            fn mul_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v * rhs)
            }
        }

        impl<T: Mul<Output = T> + Copy + 'static, S: $bound> std::ops::Mul<T> for $ty<T, S> {
            type Output = T;

            fn mul(self, rhs: T) -> Self::Output {
                self.with(|v| *v * rhs)
            }
        }

        impl<T: Div<Output = T> + Copy + 'static, S: $bound> std::ops::DivAssign<T> for $ty<T, S> {
            fn div_assign(&mut self, rhs: T) {
                self.with_mut(|v| *v = *v / rhs)
            }
        }

        impl<T: Div<Output = T> + Copy + 'static, S: $bound> std::ops::Div<T> for $ty<T, S> {
            type Output = T;

            fn div(self, rhs: T) -> Self::Output {
                self.with(|v| *v / rhs)
            }
        }

        impl<T: 'static, S: $vec_bound> $ty<Vec<T>, S> {
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
    };
}

read_impls!(CopyValue, Storage<T>);

impl<T: 'static, S: Storage<Vec<T>>> CopyValue<Vec<T>, S> {
    /// Read a value from the inner vector.
    pub fn get(&self, index: usize) -> Option<<S::Ref as Mappable<Vec<T>>>::Mapped<T>> {
        S::Ref::try_map(self.read(), move |v| v.get(index))
    }
}

impl<T: 'static, S: Storage<Option<T>>> CopyValue<Option<T>, S> {
    /// Unwraps the inner value and clones it.
    pub fn unwrap(&self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone()).unwrap()
    }

    /// Attempts to read the inner value of the Option.
    pub fn as_ref(&self) -> Option<<S::Ref as Mappable<Option<T>>>::Mapped<T>> {
        S::Ref::try_map(self.read(), |v| v.as_ref())
    }
}

write_impls!(CopyValue, Storage<T>, Storage<Vec<T>>);

impl<T: 'static, S: Storage<Option<T>>> CopyValue<Option<T>, S> {
    /// Takes the value out of the Option.
    pub fn take(&self) -> Option<T> {
        self.with_mut(|v| v.take())
    }

    /// Replace the value in the Option.
    pub fn replace(&self, value: T) -> Option<T> {
        self.with_mut(|v| v.replace(value))
    }

    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    pub fn get_or_insert(&self, default: T) -> <S::Ref as Mappable<Option<T>>>::Mapped<T> {
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    pub fn get_or_insert_with(
        &self,
        default: impl FnOnce() -> T,
    ) -> <S::Ref as Mappable<Option<T>>>::Mapped<T> {
        let borrow = self.read();
        if borrow.is_none() {
            drop(borrow);
            self.with_mut(|v| *v = Some(default()));
            S::Ref::map(self.read(), |v| v.as_ref().unwrap())
        } else {
            S::Ref::map(borrow, |v| v.as_ref().unwrap())
        }
    }
}

read_impls!(Signal, Storage<SignalData<T>>);

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> Signal<Vec<T>, S> {
    /// Read a value from the inner vector.
    pub fn get(
        &self,
        index: usize,
    ) -> Option<
        <<<S as Storage<SignalData<Vec<T>>>>::Ref as Mappable<SignalData<Vec<T>>>>::Mapped<Vec<T>> as Mappable<
            Vec<T>,
        >>::Mapped<T>,
    >{
        <<S as Storage<SignalData<Vec<T>>>>::Ref as Mappable<SignalData<Vec<T>>>>::Mapped::<Vec<T>>::try_map(self.read(), move |v| v.get(index))
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
    pub fn as_ref(
        &self,
    ) -> Option<
        <<<S as Storage<SignalData<Option<T>>>>::Ref as Mappable<SignalData<Option<T>>>>::Mapped<
            Option<T>,
        > as Mappable<Option<T>>>::Mapped<T>,
    > {
        <<S as Storage<SignalData<Option<T>>>>::Ref as Mappable<SignalData<Option<T>>>>::Mapped::<
            Option<T>,
        >::try_map(self.read(), |v| v.as_ref())
    }
}

write_impls!(Signal, Storage<SignalData<T>>, Storage<SignalData<Vec<T>>>);

impl<T: 'static, S: Storage<SignalData<Option<T>>>> Signal<Option<T>, S> {
    /// Takes the value out of the Option.
    pub fn take(&self) -> Option<T> {
        self.with_mut(|v| v.take())
    }

    /// Replace the value in the Option.
    pub fn replace(&self, value: T) -> Option<T> {
        self.with_mut(|v| v.replace(value))
    }

    /// Gets the value out of the Option, or inserts the given value if the Option is empty.
    pub fn get_or_insert(&self, default: T) -> <<S::Ref as Mappable<SignalData<Option<T>>>>::Mapped<Option<T>> as Mappable<Option<T>>>::Mapped<T>{
        self.get_or_insert_with(|| default)
    }

    /// Gets the value out of the Option, or inserts the value returned by the given function if the Option is empty.
    pub fn get_or_insert_with(
        &self,
        default: impl FnOnce() -> T,
    ) -><<S::Ref as Mappable<SignalData<Option<T>>>>::Mapped<Option<T>> as Mappable<Option<T>>>::Mapped<T>{
        let borrow = self.read();
        if borrow.is_none() {
            drop(borrow);
            self.with_mut(|v| *v = Some(default()));
            <S::Ref as Mappable<SignalData<Option<T>>>>::Mapped::<Option<T>>::map(
                self.read(),
                |v| v.as_ref().unwrap(),
            )
        } else {
            <S::Ref as Mappable<SignalData<Option<T>>>>::Mapped::<Option<T>>::map(borrow, |v| {
                v.as_ref().unwrap()
            })
        }
    }
}

read_impls!(ReadOnlySignal, Storage<SignalData<T>>);

/// An iterator over the values of a `CopyValue<Vec<T>>`.
pub struct CopyValueIterator<T: 'static, S: Storage<Vec<T>>> {
    index: usize,
    value: CopyValue<Vec<T>, S>,
}

impl<T: Clone, S: Storage<Vec<T>>> Iterator for CopyValueIterator<T, S> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index).map(|v| v.clone())
    }
}

impl<T: Clone + 'static, S: Storage<Vec<T>>> IntoIterator for CopyValue<Vec<T>, S> {
    type IntoIter = CopyValueIterator<T, S>;

    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        CopyValueIterator {
            index: 0,
            value: self,
        }
    }
}

impl<T: 'static, S: Storage<Vec<T>>> CopyValue<Vec<T>, S> {
    /// Write to an element in the inner vector.
    pub fn get_mut(&self, index: usize) -> Option<<S::Mut as MappableMut<Vec<T>>>::Mapped<T>> {
        S::Mut::try_map(self.write(), |v: &mut Vec<T>| v.get_mut(index))
    }
}

impl<T: 'static, S: Storage<Option<T>>> CopyValue<Option<T>, S> {
    /// Deref the inner value mutably.
    pub fn as_mut(
        &self,
    ) -> Option<<<S as Storage<Option<T>>>::Mut as MappableMut<Option<T>>>::Mapped<T>> {
        S::Mut::try_map(self.write(), |v: &mut Option<T>| v.as_mut())
    }
}

/// An iterator over items in a `Signal<Vec<T>>`.
pub struct SignalIterator<T: 'static, S: Storage<SignalData<Vec<T>>>> {
    index: usize,
    value: Signal<Vec<T>, S>,
}

impl<T: Clone, S: Storage<SignalData<Vec<T>>>> Iterator for SignalIterator<T, S> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.value.get(index).map(|v| v.clone())
    }
}

impl<T: Clone + 'static, S: Storage<SignalData<Vec<T>>>> IntoIterator for Signal<Vec<T>, S> {
    type IntoIter = SignalIterator<T, S>;

    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        SignalIterator {
            index: 0,
            value: self,
        }
    }
}

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> Signal<Vec<T>, S>
where
    <<S as Storage<SignalData<std::vec::Vec<T>>>>::Mut as MappableMut<
        SignalData<std::vec::Vec<T>>,
    >>::Mapped<std::vec::Vec<T>>: MappableMut<std::vec::Vec<T>>,
{
    /// Returns a reference to an element or `None` if out of bounds.
    pub fn get_mut(
        &self,
        index: usize,
    ) -> Option<
        Write<
            T,
            <<<S as Storage<SignalData<Vec<T>>>>::Mut as MappableMut<SignalData<Vec<T>>>>::Mapped<
                Vec<T>,
            > as MappableMut<Vec<T>>>::Mapped<T>,
            S,
            Vec<T>,
        >,
    > {
        Write::filter_map(self.write(), |v| v.get_mut(index))
    }
}

impl<T: 'static, S: Storage<SignalData<Option<T>>>> Signal<Option<T>, S> {
    /// Returns a reference to an element or `None` if out of bounds.
    pub fn as_mut(&self) -> Option<Write<T, <<<S as Storage<SignalData<Option<T>>>>::Mut as MappableMut<SignalData<Option<T>>>>::Mapped<Option<T>> as MappableMut<Option<T>>>::Mapped<T>, S, Option<T>>>{
        Write::filter_map(self.write(), |v| v.as_mut())
    }
}
