use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicUsize};

use blitz_dom::{Document, Widget};
use dioxus_core::{AttributeValue, IntoAttributeValue};

#[derive(Clone, PartialEq)]
pub struct SubDocumentAttr(WriteOnceAttr<Box<dyn Document>>);

impl SubDocumentAttr {
    /// Accepts any [`Document`] implementation, e.g. a plain [`BaseDocument`](blitz_dom::BaseDocument).
    pub fn new(doc: impl Document) -> Self {
        let id = doc.id();
        Self(WriteOnceAttr::new(id, Box::new(doc) as Box<dyn Document>))
    }
}

impl IntoAttributeValue for SubDocumentAttr {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Any(Rc::new(self.0))
    }
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, PartialEq)]
pub struct CustomWidgetAttr(WriteOnceAttr<Box<dyn Widget>>);

impl CustomWidgetAttr {
    pub fn new<T: Widget + 'static>(widget: T) -> Self {
        let id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let boxed = Box::new(widget) as Box<dyn Widget>;
        Self(WriteOnceAttr::new(id, boxed))
    }
}

impl IntoAttributeValue for CustomWidgetAttr {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Any(Rc::new(self.0))
    }
}

pub(crate) struct WriteOnceAttr<T> {
    id: usize,
    value: Rc<RefCell<Option<T>>>,
}

impl<T> WriteOnceAttr<T> {
    pub(crate) fn new(id: usize, value: T) -> Self {
        let value = Rc::new(RefCell::new(Some(value)));
        Self { id, value }
    }
    pub(crate) fn take(&self) -> Option<T> {
        self.value.borrow_mut().take()
    }
}

impl<T> Clone for WriteOnceAttr<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: Rc::clone(&self.value),
        }
    }
}

impl<T> PartialEq for WriteOnceAttr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
