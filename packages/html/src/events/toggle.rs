use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    ToggleEvent: [
        ///
        ontoggle
    ];
}

pub struct ToggleEvent {}

impl dioxus_core::UiEvent for ToggleEvent {}
