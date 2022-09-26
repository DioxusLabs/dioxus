use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    SelectionEvent: [
        /// onselect
        onselect
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SelectionEvent {}

impl dioxus_core::UiEvent for SelectionEvent {}
