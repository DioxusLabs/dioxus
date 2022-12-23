use dioxus_core::Event;

pub type ToggleEvent = Event<ToggleData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleData {}

impl_event! {
    ToggleData;

    /// ontoggle
    ontoggle
}
