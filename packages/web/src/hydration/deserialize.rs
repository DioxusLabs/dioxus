use std::cell::{Cell, RefCell};
use std::io::Cursor;

use serde::de::DeserializeOwned;

thread_local! {
    static SERVER_DATA: RefCell<Option<HTMLDataCursor>> = const { RefCell::new(None) };
}

/// Try to take the next item from the server data cursor. This will only be set during the first run of a component before hydration.
/// This will return `None` if no data was pushed for this instance or if serialization fails
// TODO: evan better docs
pub fn take_server_data<T: DeserializeOwned>() -> Result<Option<T>, TakeDataError> {
    SERVER_DATA.with_borrow(|data| match data.as_ref() {
        Some(data) => data.take(),
        None => Err(TakeDataError::DataNotAvailable),
    })
}

/// Run a closure with the server data
pub(crate) fn with_server_data<O>(server_data: HTMLDataCursor, f: impl FnOnce() -> O) -> O {
    // Set the server data that will be used during hydration
    set_server_data(server_data);
    let out = f();
    // Hydrating the suspense node **should** eat all the server data, but just in case, remove it
    remove_server_data();
    out
}

fn set_server_data(data: HTMLDataCursor) {
    SERVER_DATA.with_borrow_mut(|server_data| *server_data = Some(data));
}

fn remove_server_data() {
    SERVER_DATA.with_borrow_mut(|server_data| server_data.take());
}

/// Data that is deserialized from the server during hydration
pub(crate) struct HTMLDataCursor {
    data: Vec<Option<Vec<u8>>>,
    index: Cell<usize>,
}

impl HTMLDataCursor {
    pub(crate) fn from_serialized(data: &[u8]) -> Self {
        let deserialized = ciborium::from_reader(Cursor::new(data)).unwrap();
        Self::new(deserialized)
    }

    fn new(data: Vec<Option<Vec<u8>>>) -> Self {
        Self {
            data,
            index: Cell::new(0),
        }
    }

    pub fn take<T: DeserializeOwned>(&self) -> Result<Option<T>, TakeDataError> {
        let current = self.index.get();
        if current >= self.data.len() {
            tracing::trace!(
                "Tried to take more data than was available, len: {}, index: {}; This is normal if the server function was started on the client, but may indicate a bug if the server function result should be deserialized from the server",
                self.data.len(),
                current
            );
            return Err(TakeDataError::DataNotAvailable);
        }
        let bytes = self.data[current].as_ref();
        self.index.set(current + 1);
        match bytes {
            Some(bytes) => match ciborium::from_reader(Cursor::new(bytes)) {
                Ok(x) => Ok(Some(x)),
                Err(e) => {
                    tracing::error!("Error deserializing data: {:?}", e);
                    Err(TakeDataError::DeserializationError(e))
                }
            },
            None => Ok(None),
        }
    }
}

/// An error that can occur when trying to take data from the server
#[derive(Debug)]
pub enum TakeDataError {
    /// Deserializing the data failed
    DeserializationError(ciborium::de::Error<std::io::Error>),
    /// No data was available
    DataNotAvailable,
}

impl std::fmt::Display for TakeDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeserializationError(e) => write!(f, "DeserializationError: {}", e),
            Self::DataNotAvailable => write!(f, "DataNotAvailable"),
        }
    }
}

impl std::error::Error for TakeDataError {}
