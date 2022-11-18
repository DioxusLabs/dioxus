use dioxus_core::UiEvent;

pub type SelectionEvent = UiEvent<SelectionData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SelectionData {}
