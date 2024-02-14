use crate::copy_value::CopyValue;
use crate::read::Readable;
use crate::signal::Signal;
use crate::write::Writable;
use crate::{GlobalMemo, GlobalSignal, MappedSignal, ReadOnlySignal, SignalData};
use generational_box::{AnyStorage, Storage};

use std::{
    fmt::{Debug, Display},
    ops::{Add, Div, Mul, Sub},
};

macro_rules! default_impl {
    ($ty:ident $(: $extra_bounds:path)? $(, $bound_ty:ident : $bound:path, $vec_bound_ty:ident : $vec_bound:path)?) => {
        $(
            impl<T: Default + 'static, $bound_ty: $bound> Default for $ty<T, $bound_ty> {
                #[track_caller]
                fn default() -> Self {
                    Self::new_maybe_sync(Default::default())
                }
            }
        )?
    }
}

macro_rules! read_impls {
    ($ty:ident $(: $extra_bounds:path)? $(, $bound_ty:ident : $bound:path, $vec_bound_ty:ident : $vec_bound:path)?) => {
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
default_impl!(CopyValue, S: Storage<T>, S: Storage<Vec<T>>);
write_impls!(CopyValue, Storage<T>, Storage<Vec<T>>);

impl<T: 'static, S: Storage<T>> Clone for CopyValue<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, S: Storage<T>> Copy for CopyValue<T, S> {}

read_impls!(Signal, S: Storage<SignalData<T>>, S: Storage<SignalData<Vec<T>>>);
default_impl!(Signal, S: Storage<SignalData<T>>, S: Storage<SignalData<Vec<T>>>);
write_impls!(Signal, Storage<SignalData<T>>, Storage<SignalData<Vec<T>>>);

impl<T: 'static, S: Storage<SignalData<T>>> Clone for Signal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Copy for Signal<T, S> {}

read_impls!(
    ReadOnlySignal,
    S: Storage<SignalData<T>>,
    S: Storage<SignalData<Vec<T>>>
);
default_impl!(
    ReadOnlySignal,
    S: Storage<SignalData<T>>,
    S: Storage<SignalData<Vec<T>>>
);

impl<T: 'static, S: Storage<SignalData<T>>> Clone for ReadOnlySignal<T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static, S: Storage<SignalData<T>>> Copy for ReadOnlySignal<T, S> {}

read_impls!(GlobalSignal);
default_impl!(GlobalSignal);

read_impls!(GlobalMemo: PartialEq);

read_impls!(MappedSignal, S: AnyStorage, S: AnyStorage);
