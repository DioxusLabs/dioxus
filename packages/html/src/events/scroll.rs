use dioxus_core::Event;

pub type ScrollEvent = Event<ScrollData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ScrollData {}

impl_event! {
    ScrollData;

    /// onscroll
    onscroll
}
