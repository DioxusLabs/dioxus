use super::make_listener;
use crate::mouse::MouseEvent;
use dioxus_core::{Listener, NodeFactory};
use std::collections::HashMap;

event! {
    DragEvent: [
        /// ondrag
        ondrag

        /// ondragend
        ondragend

        /// ondragenter
        ondragenter

        /// ondragexit
        ondragexit

        /// ondragleave
        ondragleave

        /// ondragover
        ondragover

        /// ondragstart
        ondragstart

        /// ondrop
        ondrop
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
/// Data associated with a mouse event
///
/// Do not use the deprecated fields; they may change or become private in the future.
pub struct DragEvent {
    pub mouse: MouseEvent,
    pub files: HashMap<String, Vec<u8>>,
}

// they say dont use deref as a form of inheritence, but who cares?
impl std::ops::Deref for DragEvent {
    type Target = MouseEvent;

    fn deref(&self) -> &Self::Target {
        &self.mouse
    }
}
