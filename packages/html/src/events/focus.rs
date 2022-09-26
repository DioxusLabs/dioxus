use super::make_listener;
use dioxus_core::{Listener, NodeFactory, UiEvent};

event! {
    FocusEvent: [
        /// onfocus
        onfocus

        // onfocusout
        onfocusout

        // onfocusin
        onfocusin

        /// onblur
        onblur
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct FocusEvent {/* DOMEventInner:  Send + SyncTarget relatedTarget */}

impl UiEvent for FocusEvent {}
