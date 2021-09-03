use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
};

pub(crate) struct HookList {
    vals: RefCell<Vec<InnerHook<Box<dyn Any>>>>,
    idx: Cell<usize>,
}

impl Default for HookList {
    fn default() -> Self {
        Self {
            vals: Default::default(),
            idx: Cell::new(0),
        }
    }
}

struct InnerHook<T> {
    cell: UnsafeCell<T>,
}

impl<T> InnerHook<T> {
    fn new(new: T) -> Self {
        Self {
            cell: UnsafeCell::new(new),
        }
    }
}

impl HookList {
    pub(crate) fn next<T: 'static>(&self) -> Option<&mut T> {
        self.vals.borrow().get(self.idx.get()).and_then(|inn| {
            self.idx.set(self.idx.get() + 1);
            let raw_box = unsafe { &mut *inn.cell.get() };
            raw_box.downcast_mut::<T>()
        })
    }

    pub(crate) fn push<T: 'static>(&self, new: T) {
        self.vals.borrow_mut().push(InnerHook::new(Box::new(new)))
    }

    /// This resets the internal iterator count
    /// It's okay that we've given out each hook, but now we have the opportunity to give it out again
    /// Therefore, resetting is cosudered unsafe
    ///
    /// This should only be ran by Dioxus itself before "running scope".
    /// Dioxus knows how to descened through the tree to prevent mutable aliasing.
    pub(crate) unsafe fn reset(&mut self) {
        self.idx.set(0);
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.vals.borrow().len()
    }

    #[inline]
    pub(crate) fn cur_idx(&self) -> usize {
        self.idx.get()
    }

    #[inline]
    pub(crate) fn at_end(&self) -> bool {
        self.cur_idx() >= self.len()
    }
}
