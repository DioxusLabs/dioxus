use std::cell::{Cell, RefCell};
use std::io::Cursor;

use dioxus_core::CapturedError;
use serde::de::DeserializeOwned;

thread_local! {
    static SERVER_DATA: RefCell<Option<HTMLDataCursor>> = const { RefCell::new(None) };
}

/// Try to take the next item from the server data cursor. This will only be set during the first run of a component before hydration.
/// This will return `None` if no data was pushed for this instance or if serialization fails
// TODO: evan better docs
#[track_caller]
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
    error: Option<CapturedError>,
    data: Vec<Option<Vec<u8>>>,
    #[cfg(debug_assertions)]
    debug_types: Option<Vec<String>>,
    #[cfg(debug_assertions)]
    debug_locations: Option<Vec<String>>,
    index: Cell<usize>,
}

impl HTMLDataCursor {
    pub(crate) fn from_serialized(
        data: &[u8],
        debug_types: Option<Vec<String>>,
        debug_locations: Option<Vec<String>>,
    ) -> Self {
        let deserialized = ciborium::from_reader(Cursor::new(data)).unwrap();
        Self::new(deserialized, debug_types, debug_locations)
    }

    /// Get the error if there is one
    pub(crate) fn error(&self) -> Option<CapturedError> {
        self.error.clone()
    }

    fn new(
        data: Vec<Option<Vec<u8>>>,
        #[allow(unused)] debug_types: Option<Vec<String>>,
        #[allow(unused)] debug_locations: Option<Vec<String>>,
    ) -> Self {
        let mut myself = Self {
            index: Cell::new(0),
            error: None,
            data,
            #[cfg(debug_assertions)]
            debug_types,
            #[cfg(debug_assertions)]
            debug_locations,
        };

        // The first item is always an error if it exists
        let error = myself
            .take::<Option<CapturedError>>()
            .ok()
            .flatten()
            .flatten();

        myself.error = error;

        myself
    }

    #[track_caller]
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
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        let debug_type = self
                            .debug_types
                            .as_ref()
                            .and_then(|types| types.get(current));
                        let debug_locations = self
                            .debug_locations
                            .as_ref()
                            .and_then(|locations| locations.get(current));

                        if let (Some(debug_type), Some(debug_locations)) =
                            (debug_type, debug_locations)
                        {
                            let client_type = std::any::type_name::<T>();
                            let client_location = std::panic::Location::caller();
                            // We we have debug types and a location, we can provide a more helpful error message
                            tracing::error!(
                                "Error deserializing data: {err:?}\n\nThis type was serialized on the server at {debug_locations} with the type name {debug_type}. The client failed to deserialize the type {client_type} at {client_location}.",
                            );
                            return Err(TakeDataError::DeserializationError(err));
                        }
                    }
                    // Otherwise, just log the generic deserialization error
                    tracing::error!("Error deserializing data: {:?}", err);
                    Err(TakeDataError::DeserializationError(err))
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
