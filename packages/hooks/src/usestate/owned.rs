use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    fmt::{Debug, Display},
    rc::Rc,
};
pub struct UseStateOwned<T: 'static> {
    // this will always be outdated
    pub(crate) current_val: Rc<T>,
    pub(crate) wip: Rc<RefCell<Option<T>>>,
    pub(crate) update_callback: Rc<dyn Fn()>,
    pub(crate) update_scheuled: Cell<bool>,
}

impl<T> UseStateOwned<T> {
    pub fn get(&self) -> Ref<T> {
        Ref::map(self.wip.borrow(), |x| x.as_ref().unwrap())
    }

    pub fn set(&self, new_val: T) {
        *self.wip.borrow_mut() = Some(new_val);
        (self.update_callback)();
    }

    pub fn modify(&self) -> RefMut<T> {
        RefMut::map(self.wip.borrow_mut(), |x| x.as_mut().unwrap())
    }
}

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

impl<T: Debug> Debug for UseStateOwned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.current_val)
    }
}

// enable displaty for the handle
impl<'a, T: 'static + Display> std::fmt::Display for UseStateOwned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.current_val)
    }
}

impl<'a, T: Copy + Add<T, Output = T>> Add<T> for UseStateOwned<T> {
    type Output = T;

    fn add(self, rhs: T) -> Self::Output {
        self.current_val.add(rhs)
    }
}

impl<'a, T: Copy + Add<T, Output = T>> AddAssign<T> for UseStateOwned<T> {
    fn add_assign(&mut self, rhs: T) {
        self.set(self.current_val.add(rhs));
    }
}

/// Sub
impl<'a, T: Copy + Sub<T, Output = T>> Sub<T> for UseStateOwned<T> {
    type Output = T;

    fn sub(self, rhs: T) -> Self::Output {
        self.current_val.sub(rhs)
    }
}
impl<'a, T: Copy + Sub<T, Output = T>> SubAssign<T> for UseStateOwned<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.set(self.current_val.sub(rhs));
    }
}

/// MUL
impl<'a, T: Copy + Mul<T, Output = T>> Mul<T> for UseStateOwned<T> {
    type Output = T;

    fn mul(self, rhs: T) -> Self::Output {
        self.current_val.mul(rhs)
    }
}
impl<'a, T: Copy + Mul<T, Output = T>> MulAssign<T> for UseStateOwned<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.set(self.current_val.mul(rhs));
    }
}

/// DIV
impl<'a, T: Copy + Div<T, Output = T>> Div<T> for UseStateOwned<T> {
    type Output = T;

    fn div(self, rhs: T) -> Self::Output {
        self.current_val.div(rhs)
    }
}
impl<'a, T: Copy + Div<T, Output = T>> DivAssign<T> for UseStateOwned<T> {
    fn div_assign(&mut self, rhs: T) {
        self.set(self.current_val.div(rhs));
    }
}
