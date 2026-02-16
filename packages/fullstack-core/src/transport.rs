#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use base64::Engine;
use dioxus_core::CapturedError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{cell::RefCell, io::Cursor, rc::Rc};

#[cfg(feature = "web")]
thread_local! {
    static CONTEXT: RefCell<Option<HydrationContext>> = const { RefCell::new(None) };
}

/// Data shared between the frontend and the backend for hydration
/// of server functions.
#[derive(Default, Clone)]
pub struct HydrationContext {
    #[cfg(feature = "web")]
    /// Is resolving suspense done on the client
    suspense_finished: bool,
    data: Rc<RefCell<HTMLData>>,
}

impl HydrationContext {
    /// Create a new serialize context from the serialized data
    pub fn from_serialized(
        data: &[u8],
        debug_types: Option<Vec<String>>,
        debug_locations: Option<Vec<String>>,
    ) -> Self {
        Self {
            #[cfg(feature = "web")]
            suspense_finished: false,
            data: Rc::new(RefCell::new(HTMLData::from_serialized(
                data,
                debug_types,
                debug_locations,
            ))),
        }
    }

    /// Serialize the data in the context to be sent to the client
    pub fn serialized(&self) -> SerializedHydrationData {
        self.data.borrow().serialized()
    }

    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    pub fn create_entry<T>(&self) -> SerializeContextEntry<T> {
        let entry_index = self.data.borrow_mut().create_entry();

        SerializeContextEntry {
            index: entry_index,
            context: self.clone(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Get the entry for the error in the suspense boundary
    pub fn error_entry(&self) -> SerializeContextEntry<Option<CapturedError>> {
        // The first entry is reserved for the error
        let entry_index = self.data.borrow_mut().create_entry_with_id(0);

        SerializeContextEntry {
            index: entry_index,
            context: self.clone(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Extend this data with the data from another [`HydrationContext`]
    pub fn extend(&self, other: &Self) {
        self.data.borrow_mut().extend(&other.data.borrow());
    }

    #[cfg(feature = "web")]
    /// Run a closure inside of this context
    pub fn in_context<T>(&self, f: impl FnOnce() -> T) -> T {
        CONTEXT.with(|context| {
            let old = context.borrow().clone();
            *context.borrow_mut() = Some(self.clone());
            let result = f();
            *context.borrow_mut() = old;
            result
        })
    }

    pub(crate) fn insert<T: Transportable<M>, M: 'static>(
        &self,
        id: usize,
        value: &T,
        location: &'static std::panic::Location<'static>,
    ) {
        self.data.borrow_mut().insert(id, value, location);
    }

    pub(crate) fn get<T: Transportable<M>, M: 'static>(
        &self,
        id: usize,
    ) -> Result<T, TakeDataError> {
        // If suspense is finished on the client, we can assume that the data is available
        #[cfg(feature = "web")]
        if self.suspense_finished {
            return Err(TakeDataError::DataNotAvailable);
        }
        self.data.borrow().get(id)
    }
}

/// An entry into the serialized context. The order entries are created in must be consistent
/// between the server and the client.
pub struct SerializeContextEntry<T> {
    /// The index this context will be inserted into inside the serialize context
    index: usize,
    /// The context this entry is associated with
    context: HydrationContext,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for SerializeContextEntry<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            context: self.context.clone(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> SerializeContextEntry<T> {
    /// Insert data into an entry that was created with [`HydrationContext::create_entry`]
    pub fn insert<M: 'static>(self, value: &T, location: &'static std::panic::Location<'static>)
    where
        T: Transportable<M>,
    {
        self.context.insert(self.index, value, location);
    }

    /// Grab the data from the serialize context
    pub fn get<M: 'static>(&self) -> Result<T, TakeDataError>
    where
        T: Transportable<M>,
    {
        self.context.get(self.index)
    }
}

/// Check if the client is currently rendering a component for hydration. Always returns true on the server.
pub fn is_hydrating() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        // On the client, we can check if the context is set
        CONTEXT.with(|context| context.borrow().is_some())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        true
    }
}

/// Get or insert the current serialize context. On the client, the hydration context this returns
/// will always return `TakeDataError::DataNotAvailable` if hydration of the current chunk is finished.
pub fn serialize_context() -> HydrationContext {
    // Use target_arch to distinguish server (native) from client (wasm32)
    // instead of cfg(feature = "web") because cargo feature unification
    // can enable both "web" and "server" features simultaneously.
    #[cfg(target_arch = "wasm32")]
    {
        // On the client, the hydration logic provides the context in a global
        if let Some(current_context) = CONTEXT.with(|context| context.borrow().clone()) {
            current_context
        } else {
            // If the context is not set, then suspense is not active
            HydrationContext {
                #[cfg(feature = "web")]
                suspense_finished: true,
                ..Default::default()
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // On the server each scope creates the context lazily
        dioxus_core::has_context()
            .unwrap_or_else(|| dioxus_core::provide_context(HydrationContext::default()))
    }
}

pub(crate) struct HTMLData {
    /// The position of the cursor in the data. This is only used on the client
    pub(crate) cursor: usize,
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

impl Default for HTMLData {
    fn default() -> Self {
        Self {
            cursor: 1,
            data: Vec::new(),
            #[cfg(debug_assertions)]
            debug_types: Vec::new(),
            #[cfg(debug_assertions)]
            debug_locations: Vec::new(),
        }
    }
}

impl HTMLData {
    #[allow(unused)]
    fn from_serialized(
        data: &[u8],
        debug_types: Option<Vec<String>>,
        debug_locations: Option<Vec<String>>,
    ) -> Self {
        let data = ciborium::from_reader(Cursor::new(data)).unwrap();
        Self {
            cursor: 1,
            data,
            #[cfg(debug_assertions)]
            debug_types: debug_types
                .unwrap_or_default()
                .into_iter()
                .map(Some)
                .collect(),
            #[cfg(debug_assertions)]
            debug_locations: debug_locations
                .unwrap_or_default()
                .into_iter()
                .map(Some)
                .collect(),
        }
    }

    /// Create a new entry in the data that will be sent to the client without inserting any data. Returns an id that can be used to insert data into the entry once it is ready.
    fn create_entry(&mut self) -> usize {
        let id = self.cursor;
        self.cursor += 1;
        self.create_entry_with_id(id)
    }

    fn create_entry_with_id(&mut self, id: usize) -> usize {
        while id + 1 > self.data.len() {
            self.data.push(None);
            #[cfg(debug_assertions)]
            {
                self.debug_types.push(None);
                self.debug_locations.push(None);
            }
        }
        id
    }

    /// Insert data into an entry that was created with [`Self::create_entry`]
    fn insert<T: Transportable<M>, M: 'static>(
        &mut self,
        id: usize,
        value: &T,
        #[allow(unused)] location: &'static std::panic::Location<'static>,
    ) {
        let serialized = value.transport_to_bytes();
        self.data[id] = Some(serialized);
        #[cfg(debug_assertions)]
        {
            self.debug_types[id] = Some(std::any::type_name::<T>().to_string());
            self.debug_locations[id] = Some(location.to_string());
        }
    }

    /// Get the data from the serialize context
    fn get<T: Transportable<M>, M: 'static>(&self, index: usize) -> Result<T, TakeDataError> {
        if index >= self.data.len() {
            tracing::trace!(
                "Tried to take more data than was available, len: {}, index: {}; This is normal if the server function was started on the client, but may indicate a bug if the server function result should be deserialized from the server",
                self.data.len(),
                index
            );
            return Err(TakeDataError::DataNotAvailable);
        }
        let bytes = self.data[index].as_ref();
        match bytes {
            Some(bytes) => match T::transport_from_bytes(bytes) {
                Ok(x) => Ok(x),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        let debug_type = self.debug_types.get(index);
                        let debug_locations = self.debug_locations.get(index);

                        if let (Some(Some(debug_type)), Some(Some(debug_locations))) =
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
            None => Err(TakeDataError::DataPending),
        }
    }

    /// Extend this data with the data from another [`HTMLData`]
    pub(crate) fn extend(&mut self, other: &Self) {
        // Make sure this vectors error entry exists even if it is empty
        if self.data.is_empty() {
            self.data.push(None);
            #[cfg(debug_assertions)]
            {
                self.debug_types.push(None);
                self.debug_locations.push(None);
            }
        }

        let mut other_data_iter = other.data.iter().cloned();
        #[cfg(debug_assertions)]
        let mut other_debug_types_iter = other.debug_types.iter().cloned();
        #[cfg(debug_assertions)]
        let mut other_debug_locations_iter = other.debug_locations.iter().cloned();

        // Merge the error entry from the other context
        if let Some(Some(other_error)) = other_data_iter.next() {
            self.data[0] = Some(other_error.clone());
            #[cfg(debug_assertions)]
            {
                self.debug_types[0] = other_debug_types_iter.next().unwrap_or(None);
                self.debug_locations[0] = other_debug_locations_iter.next().unwrap_or(None);
            }
        }

        // Don't copy the error from the other context
        self.data.extend(other_data_iter);
        #[cfg(debug_assertions)]
        {
            self.debug_types.extend(other_debug_types_iter);
            self.debug_locations.extend(other_debug_locations_iter);
        }
    }

    /// Encode data as base64. This is intended to be used in the server to send data to the client.
    pub(crate) fn serialized(&self) -> SerializedHydrationData {
        let mut serialized = Vec::new();
        ciborium::into_writer(&self.data, &mut serialized).unwrap();

        let data = base64::engine::general_purpose::STANDARD.encode(serialized);

        #[cfg(debug_assertions)]
        let format_js_list_of_strings = |list: &[Option<String>]| {
            let body = list
                .iter()
                .map(|s| match s {
                    Some(s) => {
                        // Escape backslashes, quotes, and newlines
                        let escaped = s
                            .replace(r#"\"#, r#"\\"#)
                            .replace("\n", r#"\n"#)
                            .replace(r#"""#, r#"\""#);

                        format!(r#""{escaped}""#)
                    }
                    None => r#""unknown""#.to_string(),
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("[{}]", body)
        };

        SerializedHydrationData {
            data,
            #[cfg(debug_assertions)]
            debug_types: format_js_list_of_strings(&self.debug_types),
            #[cfg(debug_assertions)]
            debug_locations: format_js_list_of_strings(&self.debug_locations),
        }
    }
}

/// Data that was serialized on the server for hydration on the client. This includes
/// extra information about the types and sources of the serialized data in debug mode
pub struct SerializedHydrationData {
    /// The base64 encoded serialized data
    pub data: String,
    /// A list of the types of each serialized data
    #[cfg(debug_assertions)]
    pub debug_types: String,
    /// A list of the locations of each serialized data
    #[cfg(debug_assertions)]
    pub debug_locations: String,
}

/// An error that can occur when trying to take data from the server
#[derive(Debug)]
pub enum TakeDataError {
    /// Deserializing the data failed
    DeserializationError(ciborium::de::Error<std::io::Error>),
    /// No data was available
    DataNotAvailable,
    /// The server serialized a placeholder for the data, but it isn't available yet
    DataPending,
}

impl std::fmt::Display for TakeDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeserializationError(e) => write!(f, "DeserializationError: {}", e),
            Self::DataNotAvailable => write!(f, "DataNotAvailable"),
            Self::DataPending => write!(f, "DataPending"),
        }
    }
}

impl std::error::Error for TakeDataError {}

/// Create a new entry in the serialize context for the head element hydration
pub fn head_element_hydration_entry() -> SerializeContextEntry<bool> {
    serialize_context().create_entry()
}

/// A `Transportable` type can be safely transported from the server to the client, and be used for
/// hydration. Not all types can sensibly be transported, but many can. This trait makes it possible
/// to customize how types are transported which helps for non-serializable types like `dioxus_core::CapturedError`.
///
/// By default, all types that implement `Serialize` and `DeserializeOwned` are transportable.
///
/// You can also implement `Transportable` for `Result<T, dioxus_core::CapturedError>` where `T` is
/// `Serialize` and `DeserializeOwned` to allow transporting results that may contain errors.
///
/// Note that transporting a `Result<T, dioxus_core::CapturedError>` will lose various aspects of the original
/// `dioxus_core::CapturedError` such as backtraces and source errors, but will preserve the error message.
pub trait Transportable<M = ()>: 'static {
    /// Serialize the type to a byte vector for transport
    fn transport_to_bytes(&self) -> Vec<u8>;

    /// Deserialize the type from a byte slice
    fn transport_from_bytes(bytes: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>>
    where
        Self: Sized;
}

impl<T> Transportable<()> for T
where
    T: Serialize + DeserializeOwned + 'static,
{
    fn transport_to_bytes(&self) -> Vec<u8> {
        let mut serialized = Vec::new();
        ciborium::into_writer(self, &mut serialized).unwrap();
        serialized
    }

    fn transport_from_bytes(bytes: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>>
    where
        Self: Sized,
    {
        ciborium::from_reader(Cursor::new(bytes))
    }
}

#[derive(Serialize, Deserialize)]
struct TransportResultErr<T> {
    error: Result<T, CapturedError>,
}

#[doc(hidden)]
pub struct TransportViaErrMarker;

impl<T> Transportable<TransportViaErrMarker> for Result<T, anyhow::Error>
where
    T: Serialize + DeserializeOwned + 'static,
{
    fn transport_to_bytes(&self) -> Vec<u8> {
        let err = TransportResultErr {
            error: self
                .as_ref()
                .map_err(|e| CapturedError::from_display(e.to_string())),
        };

        let mut serialized = Vec::new();
        ciborium::into_writer(&err, &mut serialized).unwrap();
        serialized
    }

    fn transport_from_bytes(bytes: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>>
    where
        Self: Sized,
    {
        let err: TransportResultErr<T> = ciborium::from_reader(Cursor::new(bytes))?;
        match err.error {
            Ok(value) => Ok(Ok(value)),
            Err(captured) => Ok(Err(anyhow::Error::msg(captured.to_string()))),
        }
    }
}

#[doc(hidden)]
pub struct TransportCapturedError;
#[derive(Serialize, Deserialize)]
struct TransportError {
    error: String,
}

impl Transportable<TransportCapturedError> for CapturedError {
    fn transport_to_bytes(&self) -> Vec<u8> {
        let err = TransportError {
            error: self.to_string(),
        };

        let mut serialized = Vec::new();
        ciborium::into_writer(&err, &mut serialized).unwrap();
        serialized
    }

    fn transport_from_bytes(bytes: &[u8]) -> Result<Self, ciborium::de::Error<std::io::Error>>
    where
        Self: Sized,
    {
        let err: TransportError = ciborium::from_reader(Cursor::new(bytes))?;
        Ok(dioxus_core::CapturedError::msg::<String>(err.error))
    }
}
