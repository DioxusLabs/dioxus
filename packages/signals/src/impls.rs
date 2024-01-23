use crate::read::Readable;
use crate::read::ReadableVecExt;
use crate::rt::CopyValue;
use crate::signal::Signal;
use crate::write::Writable;
use crate::{GlobalMemo, GlobalSignal, ReadOnlySignal, ReadableValueIterator, SignalData};
use generational_box::UnsyncStorage;
use generational_box::{AnyStorage, Storage};

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
    };
}

read_impls!(CopyValue, S: Storage<T>, S: Storage<Vec<T>>);
write_impls!(CopyValue, Storage<T>, Storage<Vec<T>>);

impl<T: 'static, S: Storage<Vec<T>>> IntoIterator for CopyValue<Vec<T>, S> {
    type IntoIter = ReadableValueIterator<T, Self>;

    type Item = S::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

read_impls!(Signal, S: Storage<SignalData<T>>, S: Storage<SignalData<Vec<T>>>);
write_impls!(Signal, Storage<SignalData<T>>, Storage<SignalData<Vec<T>>>);

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> IntoIterator for Signal<Vec<T>, S> {
    type IntoIter = ReadableValueIterator<T, Self>;

    type Item = S::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

read_impls!(
    ReadOnlySignal,
    S: Storage<SignalData<T>>,
    S: Storage<SignalData<Vec<T>>>
);

impl<T: 'static, S: Storage<SignalData<Vec<T>>>> IntoIterator for ReadOnlySignal<Vec<T>, S> {
    type IntoIter = ReadableValueIterator<T, Self>;

    type Item = S::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

read_impls!(GlobalSignal);

impl<T: 'static> IntoIterator for GlobalSignal<Vec<T>> {
    type IntoIter = ReadableValueIterator<T, Self>;

    type Item = <UnsyncStorage as AnyStorage>::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

read_impls!(GlobalMemo: PartialEq);

impl<T: PartialEq + 'static> IntoIterator for GlobalMemo<Vec<T>> {
    type IntoIter = ReadableValueIterator<T, Self>;

    type Item = <UnsyncStorage as AnyStorage>::Ref<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
