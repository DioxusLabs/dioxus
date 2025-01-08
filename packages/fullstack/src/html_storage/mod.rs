#![cfg(feature = "server")]

use dioxus_lib::prelude::{has_context, provide_context, use_hook};
use serde::Serialize;
use std::{cell::RefCell, rc::Rc};

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
    pub(crate) fn insert<T: Serialize>(
        &self,
        id: usize,
        value: &T,
        location: &'static std::panic::Location<'static>,
    ) {
        self.data.borrow_mut().insert(id, value, location);
    }

    /// Push resolved data into the serialized server data
    pub(crate) fn push<T: Serialize>(
        &self,
        data: &T,
        location: &'static std::panic::Location<'static>,
    ) {
        self.data.borrow_mut().push(data, location);
    }
}

pub(crate) fn use_serialize_context() -> SerializeContext {
    use_hook(serialize_context)
}

pub(crate) fn serialize_context() -> SerializeContext {
    has_context().unwrap_or_else(|| provide_context(SerializeContext::default()))
}

#[derive(Default)]
pub(crate) struct HTMLData {
    /// The data required for hydration
    pub data: Vec<Option<Vec<u8>>>,
    /// The types of each serialized data
    ///
    /// NOTE: we don't store this in the main data vec because we don't want to include it in
    /// release mode and we can't assume both the client and server are built with debug assertions
    /// matching
    #[cfg(debug_assertions)]
    pub debug_types: Vec<Option<String>>,
    /// The locations of each serialized data
    #[cfg(debug_assertions)]
    pub debug_locations: Vec<Option<String>>,
}

impl HTMLData {
    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    fn create_entry(&mut self) -> usize {
        let id = self.data.len();
        self.data.push(None);
        #[cfg(debug_assertions)]
        {
            self.debug_types.push(None);
            self.debug_locations.push(None);
        }
        id
    }

    /// Insert data into an entry that was created with [`Self::create_entry`]
    fn insert<T: Serialize>(
        &mut self,
        id: usize,
        value: &T,
        location: &'static std::panic::Location<'static>,
    ) {
        let mut serialized = Vec::new();
        ciborium::into_writer(value, &mut serialized).unwrap();
        self.data[id] = Some(serialized);
        #[cfg(debug_assertions)]
        {
            self.debug_types[id] = Some(std::any::type_name::<T>().to_string());
            self.debug_locations[id] = Some(location.to_string());
        }
    }

    /// Push resolved data into the serialized server data
    fn push<T: Serialize>(&mut self, data: &T, location: &'static std::panic::Location<'static>) {
        let mut serialized = Vec::new();
        ciborium::into_writer(data, &mut serialized).unwrap();
        self.data.push(Some(serialized));
        #[cfg(debug_assertions)]
        {
            self.debug_types
                .push(Some(std::any::type_name::<T>().to_string()));
            self.debug_locations.push(Some(location.to_string()));
        }
    }

    /// Extend this data with the data from another [`HTMLData`]
    pub(crate) fn extend(&mut self, other: &Self) {
        self.data.extend_from_slice(&other.data);
        #[cfg(debug_assertions)]
        {
            self.debug_types.extend_from_slice(&other.debug_types);
            self.debug_locations
                .extend_from_slice(&other.debug_locations);
        }
    }
}
