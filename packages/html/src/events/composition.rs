use dioxus_core::Event;

pub type CompositionEvent = Event<CompositionData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct CompositionData {
    pub data: String,
}

impl_event! [
    CompositionData;

    /// oncompositionstart
    oncompositionstart

    /// oncompositionend
    oncompositionend

    /// oncompositionupdate
    oncompositionupdate
];
