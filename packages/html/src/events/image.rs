use dioxus_core::Event;

pub type ImageEvent = Event<ImageData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    pub load_error: bool,
}

impl_event! [
    ImageData;

    /// onerror
    onerror

    /// onload
    onload
];
