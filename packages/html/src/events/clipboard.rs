use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    // DOMDataTransfer clipboardData
}

event! {
    ClipboardEvent: [
        /// Called when "copy"
        oncopy

        /// oncut
        oncut

        /// onpaste
        onpaste
    ];
}
