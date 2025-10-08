use crate::file_data::HasFileData;
use crate::FileData;
use std::{collections::HashMap, fmt::Debug, ops::Deref};

use dioxus_core::Event;

pub type FormEvent = Event<FormData>;

/* DOMEvent:  Send + SyncTarget relatedTarget */
pub struct FormData {
    inner: Box<dyn HasFormData>,
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
    pub fn values(&self) -> Vec<(String, FormValue)> {
        self.inner.values()
    }

    /// Get the first value with the given name
    pub fn get_first(&self, name: &str) -> Option<FormValue> {
        self.values()
            .into_iter()
            .find_map(|(k, v)| if k == name { Some(v) } else { None })
    }

    /// Get all values with the given name
    pub fn get(&self, name: &str) -> Vec<FormValue> {
        self.values()
            .into_iter()
            .filter_map(|(k, v)| if k == name { Some(v) } else { None })
            .collect()
    }

    /// Get the files of the form event
    pub fn files(&self) -> Vec<FileData> {
        self.inner.files()
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    /// Did this form pass its own validation?
    pub fn valid(&self) -> bool {
        !self.inner.value().is_empty()
    }
}

impl FormData {
    /// Parse the values into a struct with one field per value
    #[cfg(feature = "serialize")]
    pub fn parsed_values<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::SerializedFileData;

        let values = &self.values();

        let mut map = serde_json::Map::new();
        for (key, value) in values {
            match value {
                FormValue::Text(text) => {
                    map.insert(key.clone(), serde_json::Value::String(text.clone()));
                }
                // we create the serialized variant with no bytes
                // SerializedFileData, if given a real path, will read the bytes from disk (synchronously)
                FormValue::File(Some(file_data)) => {
                    let serialized = SerializedFileData {
                        path: file_data.path().to_owned(),
                        size: file_data.size(),
                        last_modified: file_data.last_modified(),
                        content_type: file_data.content_type(),
                        contents: None,
                    };
                    map.insert(
                        key.clone(),
                        serde_json::to_value(&serialized).unwrap_or(serde_json::Value::Null),
                    );
                }
                FormValue::File(None) => {
                    let serialized = SerializedFileData {
                        path: "".into(),
                        size: 0,
                        last_modified: 0,
                        content_type: None,
                        contents: None,
                    };
                    map.insert(
                        key.clone(),
                        serde_json::to_value(&serialized).unwrap_or(serde_json::Value::Null),
                    );
                }
            }
        }

        serde_json::from_value(serde_json::Value::Object(map))
    }
}

impl HasFileData for FormData {
    fn files(&self) -> Vec<FileData> {
        self.inner.files()
    }
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

/// A value in a form, either text or a file
#[derive(Debug, Clone, PartialEq)]
pub enum FormValue {
    Text(String),
    File(Option<FileData>),
}

impl PartialEq<str> for FormValue {
    fn eq(&self, other: &str) -> bool {
        match self {
            FormValue::Text(s) => s == other,
            FormValue::File(_f) => false,
        }
    }
}

impl PartialEq<&str> for FormValue {
    fn eq(&self, other: &&str) -> bool {
        match self {
            FormValue::Text(s) => s == other,
            FormValue::File(_f) => false,
        }
    }
}

/// An object that has all the data for a form event
pub trait HasFormData: HasFileData + std::any::Any {
    fn value(&self) -> String;

    fn valid(&self) -> bool;

    fn values(&self) -> Vec<(String, FormValue)>;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "serialize")]
pub use serialize::*;

#[cfg(feature = "serialize")]
mod serialize {
    use crate::SerializedFileData;

    use super::*;

    /// A serialized form data object
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedFormData {
        #[serde(default)]
        pub value: String,

        #[serde(default)]
        pub values: Vec<SerializedFormObject>,

        #[serde(default)]
        pub valid: bool,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    pub struct SerializedFormObject {
        pub key: String,
        pub text: Option<String>,
        pub file: Option<SerializedFileData>,
    }

    #[cfg(feature = "serialize")]
    impl SerializedFormData {
        /// Create a new serialized form data object
        pub fn new(value: String, values: Vec<SerializedFormObject>) -> Self {
            Self {
                value,
                values,
                valid: true,
            }
        }

        /// Create a new serialized form data object from a traditional form data object
        pub async fn async_from(data: &FormData) -> Result<Self, dioxus_core::Error> {
            todo!()
            // Ok(Self {
            //     value: data.value(),
            //     values: data.values(),
            //     valid: data.valid(),
            //     files: {
            //         let mut out = vec![];
            //         for file in data.files() {
            //             out.push(SerializedFileData {
            //                 name: file.name(),
            //                 size: file.size(),
            //                 content_type: file.content_type(),
            //                 last_modified: file.last_modified(),
            //                 path: file.path(),
            //                 contents: file.read_bytes().await?,
            //             });
            //         }
            //         out
            //     },
            // })
        }

        fn from_lossy(data: &FormData) -> Self {
            let values = data
                .values()
                .iter()
                .map(|(k, v)| {
                    todo!()
                    // let sv = match v {
                    //     FormValue::Text(s) => SerializedFormValue::Text(s.clone()),
                    //     FormValue::File(f) => {
                    //         if let Some(sf) = f.inner().downcast_ref::<SerializedFileData>() {
                    //             SerializedFormValue::File(sf.clone())
                    //         } else {
                    //             SerializedFormValue::Text(String::new())
                    //         }
                    //     }
                    // };
                    // (k.clone(), sv)
                })
                .collect();

            Self {
                values,
                value: data.value(),
                valid: data.valid(),
            }
        }
    }

    impl HasFormData for SerializedFormData {
        fn value(&self) -> String {
            self.value.clone()
        }

        fn values(&self) -> Vec<(String, FormValue)> {
            todo!()
        }

        fn valid(&self) -> bool {
            self.valid
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    impl HasFileData for SerializedFormData {
        fn files(&self) -> Vec<FileData> {
            todo!()
            // self.values
            //     .iter()
            //     .flat_map(|(_, v)| match v {
            //         SerializedFormValue::File(f) => Some(FileData::new(f.clone())),
            //         _ => None,
            //     })
            //     .collect()
        }
    }

    impl serde::Serialize for FormData {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            SerializedFormData::from_lossy(self).serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for FormData {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let data = SerializedFormData::deserialize(deserializer)?;
            Ok(Self {
                inner: Box::new(data),
            })
        }
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
