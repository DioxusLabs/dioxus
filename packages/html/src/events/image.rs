use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    ImageData: [];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ImageData {
    pub load_error: bool,
}
