#![allow(unused)]

use std::sync::atomic::AtomicUsize;

use serde::{de::DeserializeOwned, Serialize};

pub(crate) mod deserialize;

pub(crate) mod serialize;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub(crate) struct HTMLData {
    pub data: Vec<Vec<u8>>,
}

impl HTMLData {
    pub(crate) fn push<T: Serialize>(&mut self, value: &T) {
        let serialized = postcard::to_allocvec(value).unwrap();
        self.data.push(serialized);
    }

    pub(crate) fn cursor(self) -> HTMLDataCursor {
        HTMLDataCursor {
            data: self.data,
            index: AtomicUsize::new(0),
        }
    }
}

pub(crate) struct HTMLDataCursor {
    data: Vec<Vec<u8>>,
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
        let mut cursor = &self.data[current];
        self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        match postcard::from_bytes(cursor) {
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
    use postcard::to_allocvec;

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
            let mut as_string: Vec<u8> = Vec::new();
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
            serialize::serde_to_writable(&data, &mut as_string).unwrap();

            println!("{:?}", as_string);
            println!(
                "original size: {}",
                std::mem::size_of::<Data>() * data.len()
            );
            println!("serialized size: {}", to_allocvec(&data).unwrap().len());
            println!("compressed size: {}", as_string.len());

            let decoded: Vec<Data> = deserialize::serde_from_bytes(&as_string).unwrap();
            assert_eq!(data, decoded);
        }
    }
}
