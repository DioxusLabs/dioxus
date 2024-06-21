#![allow(unused)]
use base64::Engine;
use dioxus_lib::prelude::{has_context, provide_context, use_hook};
use serialize::serde_to_writable;
use std::{cell::RefCell, io::Cursor, rc::Rc, sync::atomic::AtomicUsize};

use base64::engine::general_purpose::STANDARD;
use serde::{de::DeserializeOwned, Serialize};

pub(crate) mod deserialize;
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
    use_hook(|| has_context().unwrap_or_else(|| provide_context(SerializeContext::default())))
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

    pub(crate) fn cursor(self) -> HTMLDataCursor {
        HTMLDataCursor {
            data: self.data,
            index: AtomicUsize::new(0),
        }
    }
}

pub(crate) struct HTMLDataCursor {
    data: Vec<Option<Vec<u8>>>,
    index: AtomicUsize,
}

impl HTMLDataCursor {
    pub fn take<T: DeserializeOwned>(&self) -> Option<T> {
        let current = self.index.load(std::sync::atomic::Ordering::SeqCst);
        if current >= self.data.len() {
            tracing::error!(
                "Tried to take more data than was available, len: {}, index: {}",
                self.data.len(),
                current
            );
            return None;
        }
        let mut cursor = self.data[current].as_ref()?;
        self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        match ciborium::from_reader(Cursor::new(cursor)) {
            Ok(x) => Some(x),
            Err(e) => {
                tracing::error!("Error deserializing data: {:?}", e);
                None
            }
        }
    }
}

#[test]
fn serialized_and_deserializes() {
    use ciborium::{from_reader, into_writer};

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct Data {
        a: u32,
        b: String,
        bytes: Vec<u8>,
        nested: Nested,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct Nested {
        a: u32,
        b: u16,
        c: u8,
    }

    for x in 0..10usize {
        for y in 0..10 {
            let data = vec![
                Data {
                    a: x as u32,
                    b: "hello".to_string(),
                    bytes: vec![0; x],
                    nested: Nested {
                        a: 1,
                        b: x as u16,
                        c: 3
                    },
                };
                y
            ];

            println!(
                "original size: {}",
                std::mem::size_of::<Data>() * data.len()
            );
            let mut storage = HTMLData::default();
            storage.push(&data);
            println!(
                "serialized size: {}",
                storage.data[0].as_ref().unwrap().len()
            );
            let mut as_string = String::new();
            serde_to_writable(&data, &mut as_string).unwrap();
            println!("compressed size: {}", as_string.len());

            let decoded: Vec<Data> = deserialize::serde_from_bytes(as_string.as_bytes()).unwrap();
            assert_eq!(data, decoded);
        }
    }
}
