use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
};

type UnsafeInnerHookState = UnsafeCell<Box<dyn Any>>;
type HookCleanup = Box<dyn FnOnce(Box<dyn Any>)>;

/// An abstraction over internally stored data using a hook-based memory layout.
///
/// Hooks are allocated using Boxes and then our stored references are given out.
///
/// It's unsafe to "reset" the hooklist, but it is safe to add hooks into it.
///
/// Todo: this could use its very own bump arena, but that might be a tad overkill
#[derive(Default)]
pub(crate) struct HookList {
    vals: RefCell<Vec<(UnsafeInnerHookState, HookCleanup)>>,
    idx: Cell<usize>,
}

impl HookList {
    pub(crate) fn next<T: 'static>(&self) -> Option<&mut T> {
        self.vals.borrow().get(self.idx.get()).and_then(|inn| {
            self.idx.set(self.idx.get() + 1);
            let raw_box = unsafe { &mut *inn.0.get() };
            raw_box.downcast_mut::<T>()
        })
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

    pub(crate) fn push_hook<T: 'static>(&self, new: T, cleanup: HookCleanup) {
        self.vals
            .borrow_mut()
            .push((UnsafeCell::new(Box::new(new)), cleanup))
    }

    pub(crate) fn len(&self) -> usize {
        self.vals.borrow().len()
    }

    pub(crate) fn cur_idx(&self) -> usize {
        self.idx.get()
    }

    pub(crate) fn at_end(&self) -> bool {
        self.cur_idx() >= self.len()
    }

    pub fn clear_hooks(&mut self) {
        self.vals
            .borrow_mut()
            .drain(..)
            .for_each(|(state, cleanup)| cleanup(state.into_inner()));
    }
}
