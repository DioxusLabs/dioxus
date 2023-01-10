use crate::nodes::RenderReturn;
use bumpalo::Bump;
use std::cell::{Cell, UnsafeCell};

pub(crate) struct BumpFrame {
    pub bump: UnsafeCellWith<Bump>,
    pub node: Cell<*const RenderReturn<'static>>,
}

impl BumpFrame {
    pub(crate) fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);
        Self {
            bump: UnsafeCellWith::new(bump),
            node: Cell::new(std::ptr::null()),
        }
    }

    /// Creates a new lifetime out of thin air
    pub(crate) unsafe fn try_load_node<'b>(&self) -> Option<&'b RenderReturn<'b>> {
        let node = self.node.get();

        if node.is_null() {
            return None;
        }

        unsafe { std::mem::transmute(&*node) }
    }
}

#[derive(Debug)]
pub(crate) struct UnsafeCellWith<T>(UnsafeCell<T>);

impl<T> UnsafeCellWith<T> {
    pub(crate) const fn new(data: T) -> UnsafeCellWith<T> {
        UnsafeCellWith(UnsafeCell::new(data))
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(*const T) -> R) -> R {
        f(self.0.get())
    }

    pub(crate) fn with_mut<R>(&self, f: impl FnOnce(*mut T) -> R) -> R {
        f(self.0.get())
    }
}
