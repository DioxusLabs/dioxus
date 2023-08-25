use std::{any::Any, collections::HashMap, fmt::Debug};

use dioxus_core::Event;

pub type FormEvent = Event<FormData>;

pub struct FormData {
    inner: Box<dyn HasFormData>,
}

impl<E: HasFormData> From<E> for FormData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl PartialEq for FormData {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value() && self.values() == other.values()
    }
}

impl Debug for FormData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormEvent")
            .field("value", &self.value())
            .field("values", &self.values())
            .finish()
    }
}

impl FormData {
    /// Create a new form event
    pub fn new(event: impl HasFormData + 'static) -> Self {
        Self {
            inner: Box::new(event),
        }
    }

    /// Get the value of the form event
    pub fn value(&self) -> String {
        self.inner.value()
    }

    /// Get the values of the form event
    pub fn values(&self) -> HashMap<String, Vec<String>> {
        self.inner.values()
    }

    /// Get the files of the form event
    pub fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        self.inner.files()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

/// An object that has all the data for a form event
pub trait HasFormData: std::any::Any {
    fn value(&self) -> String {
        Default::default()
    }

    fn values(&self) -> HashMap<String, Vec<String>> {
        Default::default()
    }

    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        None
    }

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "serialize")]
/// A serialized form data object
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedFormData {
    value: String,
    values: HashMap<String, Vec<String>>,
    files: Option<std::sync::Arc<SerializedFileEngine>>,
}

#[cfg(feature = "serialize")]
impl SerializedFormData {
    /// Create a new serialized form data object
    pub fn new(
        value: String,
        values: HashMap<String, Vec<String>>,
        files: Option<std::sync::Arc<SerializedFileEngine>>,
    ) -> Self {
        Self {
            value,
            values,
            files,
        }
    }

    /// Create a new serialized form data object from a traditional form data object
    pub async fn async_from(data: &FormData) -> Self {
        Self {
            value: data.value(),
            values: data.values(),
            files: match data.files() {
                Some(files) => {
                    let mut resolved_files = HashMap::new();

                    for file in files.files() {
                        let bytes = files.read_file(&file).await;
                        resolved_files.insert(file, bytes.unwrap_or_default());
                    }

                    Some(std::sync::Arc::new(SerializedFileEngine {
                        files: resolved_files,
                    }))
                }
                None => None,
            },
        }
    }

    fn from_lossy(data: &FormData) -> Self {
        Self {
            value: data.value(),
            values: data.values(),
            files: None,
        }
    }
}

#[cfg(feature = "serialize")]
impl HasFormData for SerializedFormData {
    fn value(&self) -> String {
        self.value.clone()
    }

    fn values(&self) -> HashMap<String, Vec<String>> {
        self.values.clone()
    }

    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        self.files
            .as_ref()
            .map(|files| std::sync::Arc::clone(files) as std::sync::Arc<dyn FileEngine>)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for FormData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedFormData::from_lossy(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for FormData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedFormData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

#[cfg(feature = "serialize")]
/// A file engine that serializes files to bytes
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedFileEngine {
    files: HashMap<String, Vec<u8>>,
}

#[cfg(feature = "serialize")]
#[async_trait::async_trait(?Send)]
impl FileEngine for SerializedFileEngine {
    fn files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    async fn read_file(&self, file: &str) -> Option<Vec<u8>> {
        self.files.get(file).cloned()
    }

    async fn read_file_to_string(&self, file: &str) -> Option<String> {
        self.read_file(file)
            .await
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    }

    async fn get_native_file(&self, file: &str) -> Option<Box<dyn Any>> {
        self.read_file(file)
            .await
            .map(|val| Box::new(val) as Box<dyn Any>)
    }
}

#[async_trait::async_trait(?Send)]
pub trait FileEngine {
    // get a list of file names
    fn files(&self) -> Vec<String>;

    // read a file to bytes
    async fn read_file(&self, file: &str) -> Option<Vec<u8>>;

    // read a file to string
    async fn read_file_to_string(&self, file: &str) -> Option<String>;

    // returns a file in platform's native representation
    async fn get_native_file(&self, file: &str) -> Option<Box<dyn Any>>;
}

impl_event! {
    FormData;

    /// onchange
    onchange

    /// oninput handler
    oninput

    /// oninvalid
    oninvalid

    /// onreset
    onreset

    /// onsubmit
    onsubmit
}
