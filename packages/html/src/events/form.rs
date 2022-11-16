use std::{collections::HashMap, sync::Arc};

/* DOMEvent:  Send + SyncTarget relatedTarget */
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct FormEvent {
    pub value: String,

    pub values: HashMap<String, String>,

    #[cfg_attr(feature = "serialize", serde(skip))]
    pub files: Option<Arc<dyn FileEngine>>,
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
    FormEvent;

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
