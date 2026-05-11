use std::{cell::RefCell, rc::Rc};

use blitz_dom::{BaseDocument, PlainDocument};
use dioxus_core::{AttributeValue, IntoAttributeValue};

// Hack to get write-once semantics for an attribute
#[derive(Clone)]
pub struct SubDocumentAttr {
    id: usize,
    doc: Rc<RefCell<Option<Box<PlainDocument>>>>,
}

impl SubDocumentAttr {
    pub fn new(doc: BaseDocument) -> Self {
        let id = doc.id();
        let wrapped = Rc::new(RefCell::new(Some(Box::new(PlainDocument(doc)))));
        Self { id, doc: wrapped }
    }
    pub fn take_document(&self) -> Option<Box<PlainDocument>> {
        self.doc.borrow_mut().take()
    }
}

impl PartialEq for SubDocumentAttr {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl IntoAttributeValue for SubDocumentAttr {
    fn into_value(self) -> AttributeValue {
        AttributeValue::Any(Rc::new(self))
    }
}
