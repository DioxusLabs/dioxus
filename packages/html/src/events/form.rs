use std::{collections::HashMap, fmt::Debug};

use dioxus_core::Event;

pub type FormEvent = Event<FormData>;

/* DOMEvent:  Send + SyncTarget relatedTarget */
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct FormData {
    pub value: String,

    pub values: HashMap<String, String>,

    #[cfg_attr(feature = "serialize", serde(skip))]
    pub files: Option<Arc<dyn FileEngine>>,
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
