#![allow(unused)]
use base64::Engine;
use dioxus_lib::prelude::{has_context, provide_context, use_hook};
use serialize::serde_to_writable;
use std::{cell::RefCell, io::Cursor, rc::Rc, sync::atomic::AtomicUsize};

use base64::engine::general_purpose::STANDARD;
use serde::{de::DeserializeOwned, Serialize};

pub(crate) mod serialize;

#[derive(Default, Clone)]
pub(crate) struct SerializeContext {
    data: Rc<RefCell<HTMLData>>,
}

impl SerializeContext {
    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    pub(crate) fn create_entry(&self) -> usize {
        self.data.borrow_mut().create_entry()
    }

    /// Insert data into an entry that was created with [`Self::create_entry`]
    pub(crate) fn insert<T: Serialize>(&self, id: usize, value: &T) {
        self.data.borrow_mut().insert(id, value);
    }

    /// Push resolved data into the serialized server data
    pub(crate) fn push<T: Serialize>(&self, data: &T) {
        self.data.borrow_mut().push(data);
    }
}

pub(crate) fn use_serialize_context() -> SerializeContext {
    use_hook(serialize_context)
}

pub(crate) fn serialize_context() -> SerializeContext {
    has_context().unwrap_or_else(|| provide_context(SerializeContext::default()))
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(transparent)]
pub(crate) struct HTMLData {
    pub data: Vec<Option<Vec<u8>>>,
}

impl HTMLData {
    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    pub(crate) fn create_entry(&mut self) -> usize {
        let id = self.data.len();
        self.data.push(None);
        id
    }

    /// Insert data into an entry that was created with [`Self::create_entry`]
    pub(crate) fn insert<T: Serialize>(&mut self, id: usize, value: &T) {
        let mut serialized = Vec::new();
        ciborium::into_writer(value, &mut serialized).unwrap();
        self.data[id] = Some(serialized);
    }

    /// Push resolved data into the serialized server data
    pub(crate) fn push<T: Serialize>(&mut self, data: &T) {
        let mut serialized = Vec::new();
        ciborium::into_writer(data, &mut serialized).unwrap();
        self.data.push(Some(serialized));
    }
}
