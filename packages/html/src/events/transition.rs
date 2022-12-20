use dioxus_core::Event;

pub type TransitionEvent = Event<TransitionData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionData {
    pub property_name: String,
    pub pseudo_element: String,
    pub elapsed_time: f32,
}

impl_event! {
    TransitionData;

    /// transitionend
    ontransitionend
}
