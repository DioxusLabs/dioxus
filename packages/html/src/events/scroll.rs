use dioxus_core::UiEvent;

pub type ScrollEvent = UiEvent<ScrollData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ScrollData {}

impl_event! {
    ScrollData;

    /// onscroll
    onscroll
}
