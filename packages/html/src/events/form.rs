use crate::FileData;
use crate::file_data::HasFileData;
use std::fmt::Debug;

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
    /// Deserialize form values by input name.
    ///
    /// Text values are strings; files are [`crate::SerializedFileData`] metadata.
    /// Use `Option<SerializedFileData>` for optional uploads (`None` when unselected).
    /// Repeated names become sequences; a single value is deserialized directly.
    ///
    /// To read file contents, obtain [`FileData`] handles with [`Self::get_first`],
    /// [`Self::get`], or [`Self::files`].
    #[cfg(feature = "serialize")]
    pub fn deserialize_values<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::SerializedFileData;
        use serde_json::{Map, Value, map::Entry};

        let mut fields = Map::new();
        for (key, value) in self.values() {
            let value = match value {
                FormValue::Text(text) => Value::String(text),
                FormValue::File(file) => {
                    serde_json::to_value(file.as_ref().map(SerializedFileData::from_file_data))?
                }
            };

            match fields.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
                Entry::Occupied(mut entry) => {
                    let existing = entry.get_mut();
                    match existing {
                        Value::Array(values) => values.push(value),
                        existing => {
                            let first = existing.take();
                            *existing = Value::Array(vec![first, value]);
                        }
                    }
                }
            }
        }

        serde_json::from_value(Value::Object(fields))
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
        /// `None` for an unselected file input
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

        /// Create a serialized form data object from a form data object
        fn from_form_lossy(data: &FormData) -> Self {
            if let Some(data) = data.downcast::<SerializedFormData>() {
                return data.clone();
            }

            let values = data
                .values()
                .iter()
                .map(|(key, value)| match value {
                    FormValue::Text(s) => SerializedFormObject {
                        key: key.clone(),
                        text: Some(s.to_string()),
                        file: None,
                    },
                    FormValue::File(f) => SerializedFormObject {
                        key: key.clone(),
                        text: None,
                        file: f.as_ref().map(SerializedFileData::from_file_data),
                    },
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
            self.values
                .iter()
                .map(|v| {
                    let value = if let Some(text) = &v.text {
                        FormValue::Text(text.clone())
                    } else if let Some(file) = &v.file {
                        FormValue::File(Some(FileData::new(file.clone())))
                    } else {
                        FormValue::File(None)
                    };
                    (v.key.clone(), value)
                })
                .collect()
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
            self.values
                .iter()
                .filter_map(|v| v.file.as_ref().map(|f| FileData::new(f.clone())))
                .collect()
        }
    }

    impl serde::Serialize for FormData {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            SerializedFormData::from_form_lossy(self).serialize(serializer)
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

#[cfg(all(test, feature = "serialize"))]
mod tests {
    use super::*;
    use crate::SerializedFileData;

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn files_are_readable_from_serialized_paths() {
        use futures_util::FutureExt;

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let contents = include_str!("../../Cargo.toml");
        let data: FormData = serde_json::from_value(serde_json::json!({
            "values": [
                { "key": "description", "text": "upload" },
                {
                    "key": "files",
                    "file": {
                        "name": "Cargo.toml",
                        "path": path,
                        "size": contents.len(),
                        "last_modified": 123,
                        "content_type": "text/plain"
                    }
                }
            ]
        }))
        .unwrap();

        assert_eq!(data.get_first("description").unwrap(), "upload");
        let Some(FormValue::File(Some(file))) = data.get_first("files") else {
            panic!("file missing from form values");
        };
        assert_eq!(file, data.files()[0]);
        assert_eq!(file.name(), "Cargo.toml");
        assert_eq!(file.size(), contents.len() as u64);
        assert_eq!(file.last_modified(), 123);
        assert_eq!(file.content_type().as_deref(), Some("text/plain"));
        assert_eq!(
            file.read_string().now_or_never().unwrap().unwrap(),
            contents
        );
        assert_eq!(
            file.read_bytes().now_or_never().unwrap().unwrap().as_ref(),
            contents.as_bytes()
        );
    }

    #[test]
    fn empty_metadata_does_not_make_a_present_file_unselected() {
        let data = FormData::from(SerializedFormData::new(
            String::new(),
            vec![SerializedFormObject {
                key: "files".to_string(),
                text: None,
                file: Some(crate::SerializedFileData::empty()),
            }],
        ));

        let Some(FormValue::File(Some(file))) = data.get_first("files") else {
            panic!("present file metadata must remain selected");
        };
        assert_eq!(data.files(), vec![file.clone()]);
        assert!(file.name().is_empty());
        assert!(file.path().as_os_str().is_empty());
        assert_eq!(file.size(), 0);
        let parsed: std::collections::BTreeMap<String, Option<SerializedFileData>> =
            data.deserialize_values().unwrap();
        assert_eq!(parsed["files"], Some(SerializedFileData::empty()));
    }

    #[test]
    fn unselected_liveview_file_field_can_be_parsed() {
        let data: FormData = serde_json::from_value(serde_json::json!({
            "values": [{ "key": "uploads" }]
        }))
        .unwrap();

        assert_eq!(data.get_first("uploads"), Some(FormValue::File(None)));
        assert!(data.files().is_empty());
        let parsed: std::collections::BTreeMap<String, Option<SerializedFileData>> =
            data.deserialize_values().unwrap();
        assert_eq!(parsed["uploads"], None);
        assert!(!parsed.contains_key("missing"));
        assert!(
            data.deserialize_values::<std::collections::BTreeMap<String, SerializedFileData>>()
                .is_err()
        );
    }

    #[test]
    fn repeated_file_values_preserve_unselected_entries_when_parsed() {
        #[derive(serde::Deserialize)]
        struct Fields {
            uploads: Vec<Option<SerializedFileData>>,
            missing: Option<SerializedFileData>,
        }

        let selected = SerializedFileData {
            name: "empty.txt".into(),
            ..SerializedFileData::empty()
        };
        let data = FormData::new(SerializedFormData::new(
            String::new(),
            [None, Some(selected.clone()), None]
                .into_iter()
                .map(|file| SerializedFormObject {
                    key: "uploads".into(),
                    text: None,
                    file,
                })
                .collect(),
        ));
        let parsed: Fields = data.deserialize_values().unwrap();
        assert_eq!(parsed.uploads, vec![None, Some(selected), None]);
        assert_eq!(parsed.missing, None);
    }

    #[test]
    fn named_file_without_a_path_remains_selected_after_form_serialization() {
        let form = FormData::new(SerializedFormData::new(
            String::new(),
            vec![SerializedFormObject {
                key: "upload".into(),
                text: None,
                file: Some(SerializedFileData {
                    name: "empty.txt".into(),
                    ..SerializedFileData::empty()
                }),
            }],
        ));
        let json = serde_json::to_value(form).unwrap();
        assert_eq!(json["values"][0]["file"]["name"], "empty.txt");
        let form: FormData = serde_json::from_value(json).unwrap();
        let Some(FormValue::File(Some(file))) = form.get_first("upload") else {
            panic!("a named zero-byte file should remain selected");
        };
        assert_eq!(file.name(), "empty.txt");
        assert!(file.path().as_os_str().is_empty());
        assert_eq!(file.size(), 0);
    }
}
