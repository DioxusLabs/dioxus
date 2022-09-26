use super::make_listener;
use dioxus_core::{Listener, NodeFactory, UiEvent};
use std::cell::Cell;

event! {
    AnimationEvent: [
        /// onanimationstart
        onanimationstart

        /// onanimationend
        onanimationend

        /// onanimationiteration
        onanimationiteration
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct AnimationEvent {
    pub animation_name: String,
    pub pseudo_element: String,
    pub elapsed_time: f32,
    pub bubble: Cell<bool>,
}

impl UiEvent for AnimationEvent {}
