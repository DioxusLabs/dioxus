use crate::nodes::RenderReturn;
use crate::{Attribute, AttributeValue};
use bumpalo::Bump;
use std::cell::RefCell;
use std::cell::{Cell, UnsafeCell};

pub(crate) struct BumpFrame {
    pub bump: UnsafeCell<Bump>,
    pub node: Cell<*const RenderReturn<'static>>,
    pub(crate) attributes_to_drop_before_reset: RefCell<Vec<*const Attribute<'static>>>,
}

impl BumpFrame {
    pub(crate) fn new(capacity: usize) -> Self {
        let bump = Bump::with_capacity(capacity);
        Self {
            bump: UnsafeCell::new(bump),
            node: Cell::new(std::ptr::null()),
            attributes_to_drop_before_reset: Default::default(),
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

    pub(crate) fn bump(&self) -> &Bump {
        unsafe { &*self.bump.get() }
    }

    pub(crate) fn add_attribute_to_drop(&self, attribute: *const Attribute<'static>) {
        self.attributes_to_drop_before_reset
            .borrow_mut()
            .push(attribute);
    }

    pub(crate) unsafe fn reset(&self) {
        let mut attributes = self.attributes_to_drop_before_reset.borrow_mut();
        attributes.drain(..).for_each(|attribute| {
            let attribute = unsafe { &*attribute };
            if let AttributeValue::Any(l) = &attribute.value {
                _ = l.take();
            }
        });
        unsafe {
            let bump = &mut *self.bump.get();
            bump.reset();
        }
    }
}
