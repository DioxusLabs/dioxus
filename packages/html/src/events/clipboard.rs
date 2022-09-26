use super::make_listener;
use dioxus_core::{Listener, NodeFactory, UiEvent};
use std::fmt::Debug;

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

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ClipboardEvent {
    // DOMDataTransfer clipboardData
    #[cfg_attr(feature = "serialize", serde(skip))]
    pub data: Box<dyn ClipboardData>,
}

impl std::ops::Deref for ClipboardEvent {
    type Target = dyn ClipboardData;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

pub trait ClipboardData {}

impl UiEvent for ClipboardEvent {}
