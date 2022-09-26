use super::make_listener;
use dioxus_core::{Listener, NodeFactory, UiEvent};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct CompositionEvent {
    pub data: String,
}

impl UiEvent for CompositionEvent {}

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
