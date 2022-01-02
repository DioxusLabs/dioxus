use super::owned::UseStateOwned;
use std::{
    cell::{Ref, RefMut},
    fmt::{Debug, Display},
    rc::Rc,
};

pub struct UseState<'a, T: 'static>(pub(crate) &'a UseStateOwned<T>);

impl<T> Copy for UseState<'_, T> {}

impl<'a, T: 'static> Clone for UseState<'a, T> {
    fn clone(&self) -> Self {
        UseState(self.0)
    }
}

impl<T: Debug> Debug for UseState<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0.current_val)
    }
}

impl<'a, T: 'static> UseState<'a, T> {
    /// Tell the Dioxus Scheduler that we need to be processed
    pub fn needs_update(&self) {
        if !self.0.update_scheuled.get() {
            self.0.update_scheuled.set(true);
            (self.0.update_callback)();
        }
    }

    pub fn set(&self, new_val: T) {
        *self.0.wip.borrow_mut() = Some(new_val);
        self.needs_update();
    }

    pub fn get(&self) -> &'a T {
        &self.0.current_val
    }

    pub fn get_rc(&self) -> &'a Rc<T> {
        &self.0.current_val
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip(&self) -> Ref<Option<T>> {
        self.0.wip.borrow()
    }

    /// Get the current status of the work-in-progress data
    pub fn get_wip_mut(&self) -> RefMut<Option<T>> {
        self.0.wip.borrow_mut()
    }

    pub fn classic(self) -> (&'a T, Rc<dyn Fn(T)>) {
        (&self.0.current_val, self.setter())
    }

    pub fn setter(&self) -> Rc<dyn Fn(T)> {
        let slot = self.0.wip.clone();
        Rc::new(move |new| *slot.borrow_mut() = Some(new))
    }

    pub fn wtih(&self, f: impl FnOnce(&mut T)) {
        let mut val = self.0.wip.borrow_mut();

        if let Some(inner) = val.as_mut() {
            f(inner);
        }
    }

    pub fn for_async(&self) -> UseStateOwned<T> {
        let UseStateOwned {
            current_val,
            wip,
            update_callback,
            update_scheuled,
        } = self.0;

        UseStateOwned {
            current_val: current_val.clone(),
            wip: wip.clone(),
            update_callback: update_callback.clone(),
            update_scheuled: update_scheuled.clone(),
        }
    }
}

impl<'a, T: 'static + ToOwned<Owned = T>> UseState<'a, T> {
    /// Gain mutable access to the new value via [`RefMut`].
    ///
    /// If `modify` is called, then the component will re-render.
    ///
    /// This method is only available when the value is a `ToOwned` type.
    ///
    /// Mutable access is derived by calling "ToOwned" (IE cloning) on the current value.
    ///
    /// To get a reference to the current value, use `.get()`
    pub fn modify(self) -> RefMut<'a, T> {
        // make sure we get processed
        self.needs_update();

        // Bring out the new value, cloning if it we need to
        // "get_mut" is locked behind "ToOwned" to make it explicit that cloning occurs to use this
        RefMut::map(self.0.wip.borrow_mut(), |slot| {
            if slot.is_none() {
                *slot = Some(self.0.current_val.as_ref().to_owned());
            }
            slot.as_mut().unwrap()
        })
    }

    pub fn inner(self) -> T {
        self.0.current_val.as_ref().to_owned()
    }
}

impl<'a, T> std::ops::Deref for UseState<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseState<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.current_val)
    }
}
impl<'a, V, T: PartialEq<V>> PartialEq<V> for UseState<'a, T> {
    fn eq(&self, other: &V) -> bool {
        self.get() == other
    }
}
impl<'a, O, T: std::ops::Not<Output = O> + Copy> std::ops::Not for UseState<'a, T> {
    type Output = O;

    fn not(self) -> Self::Output {
        !*self.get()
    }
}

/*

Convenience methods for UseState.

Note!

This is not comprehensive.
This is *just* meant to make common operations easier.
*/

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

impl<'a, T: Copy + Add<T, Output = T>> Add<T> for UseState<'a, T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.0.current_val.add(rhs)
    }
}
impl<'a, T: Copy + Add<T, Output = T>> AddAssign<T> for UseState<'a, T> {
    fn add_assign(&mut self, rhs: T) {
        self.set(self.0.current_val.add(rhs));
    }
}

/// Sub
impl<'a, T: Copy + Sub<T, Output = T>> Sub<T> for UseState<'a, T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.0.current_val.sub(rhs)
    }
}
impl<'a, T: Copy + Sub<T, Output = T>> SubAssign<T> for UseState<'a, T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set(self.0.current_val.sub(rhs));
    }
}

/// MUL
impl<'a, T: Copy + Mul<T, Output = T>> Mul<T> for UseState<'a, T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.0.current_val.mul(rhs)
    }
}
impl<'a, T: Copy + Mul<T, Output = T>> MulAssign<T> for UseState<'a, T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set(self.0.current_val.mul(rhs));
    }
}

/// DIV
impl<'a, T: Copy + Div<T, Output = T>> Div<T> for UseState<'a, T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.0.current_val.div(rhs)
    }
}
impl<'a, T: Copy + Div<T, Output = T>> DivAssign<T> for UseState<'a, T> {
    fn div_assign(&mut self, rhs: T) {
        self.set(self.0.current_val.div(rhs));
    }
}
