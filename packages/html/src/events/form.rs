use std::{any::Any, collections::HashMap, fmt::Debug};

use dioxus_core::Event;
use serde::{Deserialize, Serialize};

pub type FormEvent = Event<FormData>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)] // this will serialize Text(String) -> String and VecText(Vec<String>) to Vec<String>
pub enum ValueType {
    Text(String),
    VecText(Vec<String>),
}

/* DOMEvent:  Send + SyncTarget relatedTarget */
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct FormData {
    pub value: String,

    pub values: HashMap<String, ValueType>,

    #[cfg_attr(
        feature = "serialize",
        serde(
            default,
            skip_serializing,
            deserialize_with = "deserialize_file_engine"
        )
    )]
    pub files: Option<std::sync::Arc<dyn FileEngine>>,
}

fn convert_hashmap_to_json<K, V>(hashmap: &HashMap<K, V>) -> serde_json::Result<String>
where
    K: Serialize + std::hash::Hash + Eq,
    V: Serialize,
{
    serde_json::to_string(hashmap)
}

impl FormData {
    pub fn parsed_values<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let parsed_json =
            convert_hashmap_to_json(&self.values).expect("Failed to parse values to JSON");

        serde_json::from_str(&parsed_json)
    }
}

#[cfg(feature = "serialize")]
#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedFileEngine {
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

#[cfg(feature = "serialize")]
fn deserialize_file_engine<'de, D>(
    deserializer: D,
) -> Result<Option<std::sync::Arc<dyn FileEngine>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Ok(file_engine) = SerializedFileEngine::deserialize(deserializer) else {
        return Ok(None);
    };

    let file_engine = std::sync::Arc::new(file_engine);
    Ok(Some(file_engine))
}

impl PartialEq for FormData {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.values == other.values
    }
}

impl Debug for FormData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormEvent")
            .field("value", &self.value)
            .field("values", &self.values)
            .finish()
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
