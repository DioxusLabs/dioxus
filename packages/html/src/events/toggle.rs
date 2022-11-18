use dioxus_core::UiEvent;

pub type ToggleEvent = UiEvent<ToggleData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ToggleData {}
