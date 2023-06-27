use std::{cell::UnsafeCell, fmt::Debug};
use crate::ElementId;

// Saftey: There is no way to get references to the internal data of this struct so no refrences will be invalidated by mutating the data with a immutable reference (The same principle behind Cell)
#[derive(Debug, Default)]
pub struct BoxedCellSlice(UnsafeCell<Option<Box<[ElementId]>>>);

impl Clone for BoxedCellSlice {
    fn clone(&self) -> Self {
        Self(UnsafeCell::new(unsafe { (*self.0.get()).clone() }))
    }
}

impl BoxedCellSlice {
    pub fn last(&self) -> Option<ElementId> {
        unsafe {
            (*self.0.get())
                .as_ref()
                .and_then(|inner| inner.as_ref().last().copied())
        }
    }

    pub fn get(&self, idx: usize) -> Option<ElementId> {
        unsafe {
            (*self.0.get())
                .as_ref()
                .and_then(|inner| inner.as_ref().get(idx).copied())
        }
    }

    pub unsafe fn get_unchecked(&self, idx: usize) -> Option<ElementId> {
        (*self.0.get())
            .as_ref()
            .and_then(|inner| inner.as_ref().get(idx).copied())
    }

    pub fn set(&self, idx: usize, new: ElementId) {
        unsafe {
            if let Some(inner) = &mut *self.0.get() {
                inner[idx] = new;
            }
        }
    }

    pub fn intialize(&self, contents: Box<[ElementId]>) {
        unsafe {
            *self.0.get() = Some(contents);
        }
    }

    pub fn transfer(&self, other: &Self) {
        unsafe {
            *self.0.get() = (*other.0.get()).clone();
        }
    }

    pub fn take_from(&self, other: &Self) {
        unsafe {
            *self.0.get() = (*other.0.get()).take();
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            (*self.0.get())
                .as_ref()
                .map(|inner| inner.len())
                .unwrap_or(0)
        }
    }
}

impl<'a> IntoIterator for &'a BoxedCellSlice {
    type Item = ElementId;

    type IntoIter = BoxedCellSliceIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BoxedCellSliceIter {
            index: 0,
            borrow: self,
        }
    }
}

pub struct BoxedCellSliceIter<'a> {
    index: usize,
    borrow: &'a BoxedCellSlice,
}

impl Iterator for BoxedCellSliceIter<'_> {
    type Item = ElementId;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.borrow.get(self.index);
        if result.is_some() {
            self.index += 1;
        }
        result
    }
}
