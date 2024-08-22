use crate::file_data::HasFileData;
use std::{collections::HashMap, fmt::Debug, ops::Deref};

use dioxus_core_types::Event;

pub type FormEvent = Event<FormData>;

/// A form value that may either be a list of values or a single value
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FormValue(pub Vec<String>);

impl Deref for FormValue {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl FormValue {
    /// Convenient way to represent Value as slice
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    /// Return the first value, panicking if there are none
    pub fn as_value(&self) -> String {
        self.0.first().unwrap().clone()
    }

    /// Convert into [`Vec<String>`]
    pub fn to_vec(self) -> Vec<String> {
        self.0.clone()
    }
}

impl PartialEq<str> for FormValue {
    fn eq(&self, other: &str) -> bool {
        self.0.len() == 1 && self.0.first().map(|s| s.as_str()) == Some(other)
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
            .field("valid", &self.valid())
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

    /// Collect all the named form values from the containing form.
    ///
    /// Every input must be named!
    pub fn values(&self) -> HashMap<String, FormValue> {
        self.inner.values()
    }

    /// Get the files of the form event
    #[cfg(feature = "file-engine")]
    pub fn files(&self) -> Option<std::sync::Arc<dyn crate::file_data::FileEngine>> {
        self.inner.files()
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    /// Did this form pass its own validation?
    pub fn valid(&self) -> bool {
        self.inner.value().is_empty()
    }
}

/// An object that has all the data for a form event
pub trait HasFormData: HasFileData + std::any::Any {
    fn value(&self) -> String {
        Default::default()
    }

    fn valid(&self) -> bool {
        true
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
    #[serde(default)]
    value: String,

    #[serde(default)]
    values: HashMap<String, FormValue>,

    #[serde(default)]
    valid: bool,

    #[cfg(feature = "file-engine")]
    #[serde(default)]
    files: Option<crate::file_data::SerializedFileEngine>,
}

#[cfg(feature = "serialize")]
impl SerializedFormData {
    /// Create a new serialized form data object
    pub fn new(value: String, values: HashMap<String, FormValue>) -> Self {
        Self {
            value,
            values,
            valid: true,
            #[cfg(feature = "file-engine")]
            files: None,
        }
    }

    #[cfg(feature = "file-engine")]
    /// Add files to the serialized form data object
    pub fn with_files(mut self, files: crate::file_data::SerializedFileEngine) -> Self {
        self.files = Some(files);
        self
    }

    /// Create a new serialized form data object from a traditional form data object
    pub async fn async_from(data: &FormData) -> Self {
        Self {
            value: data.value(),
            values: data.values(),
            valid: data.valid(),
            #[cfg(feature = "file-engine")]
            files: {
                match data.files() {
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
                }
            },
        }
    }

    fn from_lossy(data: &FormData) -> Self {
        Self {
            value: data.value(),
            values: data.values(),
            valid: data.valid(),
            #[cfg(feature = "file-engine")]
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

    fn valid(&self) -> bool {
        self.valid
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl HasFileData for SerializedFormData {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn crate::FileEngine>> {
        self.files
            .as_ref()
            .map(|files| std::sync::Arc::new(files.clone()) as _)
    }
}

#[cfg(feature = "file-engine")]
impl HasFileData for FormData {
    fn files(&self) -> Option<std::sync::Arc<dyn crate::FileEngine>> {
        self.inner.files()
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

    /// The `oninput` event is fired when the value of a `<input>`, `<select>`, or `<textarea>` element is changed.
    ///
    /// There are two main approaches to updating your input element:
    /// 1) Controlled inputs directly update the value of the input element as the user interacts with the element
    ///
    /// ```rust
    /// use dioxus::prelude::*;
    ///
    /// fn App() -> Element {
    ///     let mut value = use_signal(|| "hello world".to_string());
    ///
    ///     rsx! {
    ///         input {
    ///             // We directly set the value of the input element to our value signal
    ///             value: "{value}",
    ///             // The `oninput` event handler will run every time the user changes the value of the input element
    ///             // We can set the `value` signal to the new value of the input element
    ///             oninput: move |event| value.set(event.value())
    ///         }
    ///         // Since this is a controlled input, we can also update the value of the input element directly
    ///         button {
    ///             onclick: move |_| value.write().clear(),
    ///             "Clear"
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// 2) Uncontrolled inputs just read the value of the input element as it changes
    ///
    /// ```rust
    /// use dioxus::prelude::*;
    ///
    /// fn App() -> Element {
    ///     rsx! {
    ///         input {
    ///             // In uncontrolled inputs, we don't set the value of the input element directly
    ///             // But you can still read the value of the input element
    ///             oninput: move |event| println!("{}", event.value()),
    ///         }
    ///         // Since we don't directly control the value of the input element, we can't easily modify it
    ///     }
    /// }
    /// ```
    oninput

    /// oninvalid
    oninvalid

    /// onreset
    onreset

    /// onsubmit
    onsubmit
}
