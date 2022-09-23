use std::collections::HashMap;

use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

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

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct FormEvent {
    pub value: String,
    pub values: HashMap<String, String>,
    /* DOMEvent:  Send + SyncTarget relatedTarget */
}
