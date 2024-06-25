use std::cell::{Cell, RefCell};
use std::io::Cursor;

use serde::de::DeserializeOwned;

thread_local! {
    static SERVER_DATA: RefCell<Option<HTMLDataCursor>> = const { RefCell::new(None) };
}

/// Try to take the next item from the server data cursor. This will only be set during the first run of a component before hydration.
/// This will return `None` if no data was pushed for this instance or if serialization fails
// TODO: evan better docs
pub fn take_server_data<T: DeserializeOwned>() -> Option<T> {
    SERVER_DATA.with_borrow(|data| data.as_ref()?.take())
}

pub(crate) fn set_server_data(data: HTMLDataCursor) {
    SERVER_DATA.with_borrow_mut(|server_data| *server_data = Some(data));
}

/// Data that is deserialized from the server during hydration
pub(crate) struct HTMLDataCursor {
    data: Vec<Option<Vec<u8>>>,
    index: Cell<usize>,
}

impl HTMLDataCursor {
    pub(crate) fn from_serialized(data: &[u8]) -> Self {
        let deserialized = ciborium::from_reader(Cursor::new(data)).unwrap();
        tracing::trace!("Deserializing server data: {:?}", deserialized);
        Self::new(deserialized)
    }

    fn new(data: Vec<Option<Vec<u8>>>) -> Self {
        Self {
            data,
            index: Cell::new(0),
        }
    }

    pub fn take<T: DeserializeOwned>(&self) -> Option<T> {
        let current = self.index.get();
        if current >= self.data.len() {
            tracing::error!(
                "Tried to take more data than was available, len: {}, index: {}",
                self.data.len(),
                current
            );
            return None;
        }
        let cursor = self.data[current].as_ref()?;
        self.index.set(current + 1);
        match ciborium::from_reader(Cursor::new(cursor)) {
            Ok(x) => Some(x),
            Err(e) => {
                tracing::error!("Error deserializing data: {:?}", e);
                None
            }
        }
    }
}
