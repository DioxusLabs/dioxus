use dioxus_core::UiEvent;

pub type TransitionEvent = UiEvent<TransitionData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TransitionData {
    pub property_name: String,
    pub pseudo_element: String,
    pub elapsed_time: f32,
}
