use std::{collections::HashMap, sync::Arc};

use super::make_listener;
use dioxus_core::{Listener, NodeFactory, UiEvent};

event! {
    FormEvent: [
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
    ];

}

impl UiEvent for FormEvent {}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct FormEvent {
    pub value: String,

    pub values: HashMap<String, String>,

    #[cfg_attr(feature = "serialize", serde(skip))]
    pub files: Arc<dyn FileEngine>,
    /* DOMEvent:  Send + SyncTarget relatedTarget */
}

#[derive(Debug)]
pub struct VirtualFile {}

#[async_trait::async_trait(?Send)]
pub trait FileEngine {
    // get a list of file names
    fn files(&self) -> Vec<String>;

    // read a file to bytes
    async fn read_file(&self, file: &str) -> Option<Vec<u8>>;

    // read a file to string
    async fn read_file_to_string(&self, file: &str) -> Option<String>;
}
