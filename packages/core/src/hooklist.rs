use std::{
    any::Any,
    cell::{Cell, UnsafeCell},
};

pub struct HookList {
    vals: appendlist::AppendList<InnerHook<Box<dyn Any>>>,
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
    /// Unsafely get a mutable reference to any of the hooks
    ///
    /// This is unsafe because an &mut T might be aliased if the hook data is already borrowed/in use in the component
    ///
    /// This method should be reserved for internal methods that are guaranteed that this hook is not aliased anyhwere
    /// inside the component body, or outside into children components.
    ///
    /// This method is currently used only by the suspense system whose hook implementation guarantees that all &T is dropped
    /// before the suspense handler is ran.
    pub(crate) unsafe fn get_mut<T: 'static>(&self, idx: usize) -> Option<&mut T> {
        self.vals.get(idx).and_then(|inn| {
            let raw_box = unsafe { &mut *inn.cell.get() };
            raw_box.downcast_mut::<T>()
        })
    }

    pub(crate) fn next<T: 'static>(&self) -> Option<&mut T> {
        self.vals.get(self.idx.get()).and_then(|inn| {
            self.idx.set(self.idx.get() + 1);
            let raw_box = unsafe { &mut *inn.cell.get() };
            raw_box.downcast_mut::<T>()
        })
    }

    #[inline]
    pub(crate) fn push<T: 'static>(&self, new: T) {
        self.vals.push(InnerHook::new(Box::new(new)))
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
        self.vals.len()
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
