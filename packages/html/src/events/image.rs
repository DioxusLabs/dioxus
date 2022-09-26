use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    ImageEvent: [];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ImageEvent {
    pub load_error: bool,
}

impl dioxus_core::UiEvent for ImageEvent {}
