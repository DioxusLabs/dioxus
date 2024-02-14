use crate::file_data::FileEngine;
use crate::file_data::HasFileData;
use std::ops::Deref;
use std::{collections::HashMap, fmt::Debug};

use dioxus_core::Event;

pub type FormEvent = Event<FormData>;

/// A form value that may either be a list of values or a single value
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    // this will serialize Text(String) -> String and VecText(Vec<String>) to Vec<String>
    serde(untagged)
)]
#[derive(Debug, Clone, PartialEq)]
pub enum FormValue {
    Text(String),
    VecText(Vec<String>),
}

impl From<FormValue> for Vec<String> {
    fn from(value: FormValue) -> Self {
        match value {
            FormValue::Text(s) => vec![s],
            FormValue::VecText(vec) => vec,
        }
    }
}

impl Deref for FormValue {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl FormValue {
    /// Convenient way to represent Value as slice
    pub fn as_slice(&self) -> &[String] {
        match self {
            FormValue::Text(s) => std::slice::from_ref(s),
            FormValue::VecText(vec) => vec.as_slice(),
        }
    }
    /// Convert into Vec<String>
    pub fn to_vec(self) -> Vec<String> {
        self.into()
    }
}

/* DOMEvent:  Send + SyncTarget relatedTarget */
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

    /// Get the value of the form event as a parsed type
    pub fn parsed<T>(&self) -> Result<T, T::Err>
    where
        T: std::str::FromStr,
    {
        self.value().parse()
    }

    /// Try to parse the value as a boolean
    ///
    /// Returns false if the value is not a boolean, or if it is false!
    /// Does not verify anything about the event itself, use with caution
    pub fn checked(&self) -> bool {
        self.value().parse().unwrap_or(false)
    }

    /// Get the values of the form event
    pub fn values(&self) -> HashMap<String, FormValue> {
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
pub trait HasFormData: HasFileData + std::any::Any {
    fn value(&self) -> String {
        Default::default()
    }

    fn values(&self) -> HashMap<String, FormValue> {
        Default::default()
    }

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl FormData {
    #[cfg(feature = "serialize")]
    /// Parse the values into a struct with one field per value
    pub fn parsed_values<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        use serde::Serialize;

        fn convert_hashmap_to_json<K, V>(hashmap: &HashMap<K, V>) -> serde_json::Result<String>
        where
            K: Serialize + std::hash::Hash + Eq,
            V: Serialize,
        {
            serde_json::to_string(hashmap)
        }

        let parsed_json =
            convert_hashmap_to_json(&self.values()).expect("Failed to parse values to JSON");

        serde_json::from_str(&parsed_json)
    }
}

#[cfg(feature = "serialize")]
/// A serialized form data object
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedFormData {
    value: String,
    values: HashMap<String, FormValue>,
    files: Option<crate::file_data::SerializedFileEngine>,
}

#[cfg(feature = "serialize")]
impl SerializedFormData {
    /// Create a new serialized form data object
    pub fn new(
        value: String,
        values: HashMap<String, FormValue>,
        files: Option<crate::file_data::SerializedFileEngine>,
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

                    Some(crate::file_data::SerializedFileEngine {
                        files: resolved_files,
                    })
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

    fn values(&self) -> HashMap<String, FormValue> {
        self.values.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl HasFileData for SerializedFormData {
    fn files(&self) -> Option<std::sync::Arc<dyn FileEngine>> {
        self.files
            .as_ref()
            .map(|files| std::sync::Arc::new(files.clone()) as _)
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
