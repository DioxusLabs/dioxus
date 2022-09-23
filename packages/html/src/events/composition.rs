use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct CompositionEvent {
    pub data: String,
}

event! {
    CompositionEvent: [
        /// oncompositionend
        oncompositionend

        /// oncompositionstart
        oncompositionstart

        /// oncompositionupdate
        oncompositionupdate
    ];
}
