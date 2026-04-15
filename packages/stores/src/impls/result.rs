//! `Result`-shaped projector methods.

use std::ops::DerefMut;

use crate::{ProjectScope, Projected};
use dioxus_signals::{Readable, ReadableExt};

/// Projection methods for types targeting `Result<T, E>`.
pub trait ProjectResult<T: 'static, E: 'static>:
    ProjectScope<Lens: Readable<Target = Result<T, E>>>
{
    fn is_ok(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_ok()
    }

    fn is_err(&self) -> bool {
        self.project_track_shallow();
        self.project_lens().peek_unchecked().is_err()
    }

    fn is_ok_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        self.project_track_shallow();
        match &*self.project_lens().peek_unchecked() {
            Ok(v) => {
                self.project_track();
                f(v)
            }
            Err(_) => false,
        }
    }

    fn is_err_and(&self, f: impl FnOnce(&E) -> bool) -> bool {
        self.project_track_shallow();
        match &*self.project_lens().peek_unchecked() {
            Err(e) => {
                self.project_track();
                f(e)
            }
            Ok(_) => false,
        }
    }

    fn ok(self) -> Option<Projected<Self, T>> {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T = |r| {
                r.as_ref()
                    .unwrap_or_else(|_| panic!("Tried to access `Ok` on a `Result` value"))
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T = |r| {
                r.as_mut()
                    .unwrap_or_else(|_| panic!("Tried to access `Ok` on a `Result` value"))
            };
            Some(self.project_child(0, map, map_mut))
        } else {
            None
        }
    }

    fn err(self) -> Option<Projected<Self, E>> {
        if self.is_err() {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on a `Result` value")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on a `Result` value")
                }
            };
            Some(self.project_child(1, map, map_mut))
        } else {
            None
        }
    }

    #[allow(clippy::type_complexity)]
    fn transpose(self) -> Result<Projected<Self, T>, Projected<Self, E>> {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T =
                |r| r.as_ref().unwrap_or_else(|_| panic!("unreachable"));
            let map_mut: fn(&mut Result<T, E>) -> &mut T =
                |r| r.as_mut().unwrap_or_else(|_| panic!("unreachable"));
            Ok(self.project_child(0, map, map_mut))
        } else {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("unreachable")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("unreachable")
                }
            };
            Err(self.project_child(1, map, map_mut))
        }
    }

    /// Unwrap into `Self<T>`; panics if currently `Err`.
    fn unwrap(self) -> Projected<Self, T>
    where
        E: std::fmt::Debug,
    {
        match self.transpose() {
            Ok(ok) => ok,
            Err(_) => panic!("called `unwrap` on an Err Result projection"),
        }
    }

    /// Unwrap into `Self<T>`; panics with `msg` if currently `Err`.
    fn expect(self, msg: &'static str) -> Projected<Self, T>
    where
        E: std::fmt::Debug,
    {
        match self.transpose() {
            Ok(ok) => ok,
            Err(_) => panic!("{}", msg),
        }
    }

    /// Unwrap into `Self<E>`; panics if currently `Ok`.
    fn unwrap_err(self) -> Projected<Self, E>
    where
        T: std::fmt::Debug,
    {
        match self.transpose() {
            Err(e) => e,
            Ok(_) => panic!("called `unwrap_err` on an Ok Result projection"),
        }
    }

    /// Unwrap into `Self<E>`; panics with `msg` if currently `Ok`.
    fn expect_err(self, msg: &'static str) -> Projected<Self, E>
    where
        T: std::fmt::Debug,
    {
        match self.transpose() {
            Err(e) => e,
            Ok(_) => panic!("{}", msg),
        }
    }

    /// Inspect the inner `Ok` value if present; tracks shallowly, and deeply when `Ok`.
    fn inspect(self, f: impl FnOnce(&T)) -> Self {
        self.project_track_shallow();
        if let Ok(v) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(v);
        }
        self
    }

    /// Inspect the inner `Err` value if present; tracks shallowly, and deeply when `Err`.
    fn inspect_err(self, f: impl FnOnce(&E)) -> Self {
        self.project_track_shallow();
        if let Err(e) = &*self.project_lens().peek_unchecked() {
            self.project_track();
            f(e);
        }
        self
    }

    /// Project through `Deref` on the `Ok` / `Err` variants.
    #[allow(clippy::type_complexity)]
    fn as_deref(self) -> Result<Projected<Self, T::Target>, Projected<Self, E>>
    where
        T: DerefMut,
        T::Target: 'static,
    {
        if self.is_ok() {
            let map: fn(&Result<T, E>) -> &T::Target = |r| match r {
                Ok(t) => &**t,
                Err(_) => panic!("Tried to access `Ok` on an `Err` value"),
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut T::Target = |r| match r {
                Ok(t) => &mut **t,
                Err(_) => panic!("Tried to access `Ok` on an `Err` value"),
            };
            Ok(self.project_child(0, map, map_mut))
        } else {
            let map: fn(&Result<T, E>) -> &E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on an `Ok` value")
                }
            };
            let map_mut: fn(&mut Result<T, E>) -> &mut E = |r| {
                if let Err(e) = r {
                    e
                } else {
                    panic!("Tried to access `Err` on an `Ok` value")
                }
            };
            Err(self.project_child(1, map, map_mut))
        }
    }
}

impl<T: 'static, E: 'static, P> ProjectResult<T, E> for P where
    P: ProjectScope<Lens: Readable<Target = Result<T, E>>>
{
}
