use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    TransitionEvent: [
        ///
        ontransitionend
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TransitionEvent {
    pub property_name: String,
    pub pseudo_element: String,
    pub elapsed_time: f32,
}
