//! `Option`-shaped projector methods.

use std::ops::DerefMut;

use crate::{ProjectScope, Projected};
use dioxus_signals::{Readable, ReadableExt};

/// Projection methods for types targeting `Option<T>`.
pub trait ProjectOption<T: 'static>: ProjectScope<Lens: Readable<Target = Option<T>>> {
    /// Is the option currently `Some`? Tracks shallowly.
    fn is_some(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_some()
    }

    /// Is the option currently `None`? Tracks shallowly.
    fn is_none(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_none()
    }

    /// Tracks shallowly and deeply if the option is `Some`.
    fn is_some_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        if let Some(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v)
        } else {
            false
        }
    }

    /// Tracks shallowly and deeply if the option is `Some`.
    fn is_none_or(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        if let Some(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v)
        } else {
            true
        }
    }

    /// Transpose `Self<Option<T>>` into `Option<Self<T>>`.
    fn transpose(self) -> Option<Projected<Self, T>> {
        if self.is_some() {
            let map: fn(&Option<T>) -> &T = |v| {
                v.as_ref()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            let map_mut: fn(&mut Option<T>) -> &mut T = |v| {
                v.as_mut()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Unwrap to `Self<T>`; panics if currently `None`.
    fn unwrap(self) -> Projected<Self, T> {
        self.transpose()
            .unwrap_or_else(|| panic!("called `unwrap` on a `None` Option projection"))
    }

    /// Unwrap to `Self<T>`; panics with `msg` if currently `None`.
    fn expect(self, msg: &'static str) -> Projected<Self, T> {
        self.transpose().unwrap_or_else(|| panic!("{}", msg))
    }

    /// Return a `[T]` view of the option: `&[value]` if `Some`, `&[]` if `None`.
    fn as_slice(self) -> Projected<Self, [T]>
    where
        T: Sized,
    {
        let map: fn(&Option<T>) -> &[T] = |value| value.as_slice();
        let map_mut: fn(&mut Option<T>) -> &mut [T] = |value| value.as_mut_slice();
        self.project_map(map, map_mut)
    }

    /// Project through `Deref` on the contained value.
    fn as_deref(self) -> Option<Projected<Self, T::Target>>
    where
        T: DerefMut,
        T::Target: 'static,
    {
        if self.is_some() {
            let map: fn(&Option<T>) -> &T::Target = |v| {
                (&**v
                    .as_ref()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value")))
                    as &T::Target
            };
            let map_mut: fn(&mut Option<T>) -> &mut T::Target = |v| {
                &mut **v
                    .as_mut()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Filter the option by a predicate. Always tracks shallowly; tracks deeply when `Some`.
    fn filter(self, f: impl FnOnce(&T) -> bool) -> Option<Projected<Self, T>> {
        if self.is_some_and(f) {
            let map: fn(&Option<T>) -> &T = |v| {
                v.as_ref()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            let map_mut: fn(&mut Option<T>) -> &mut T = |v| {
                v.as_mut()
                    .unwrap_or_else(|| panic!("Tried to access `Some` on an Option value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    /// Peek at the inner value if `Some`; tracks shallowly, and deeply when `Some`.
    fn inspect(self, f: impl FnOnce(&T)) -> Self {
        self.project_track_shallow();
        if let Some(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v);
        }
        self
    }
}

impl<T: 'static, P> ProjectOption<T> for P where P: ProjectScope<Lens: Readable<Target = Option<T>>> {}
