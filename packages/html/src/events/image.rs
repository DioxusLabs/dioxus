use dioxus_core::UiEvent;

pub type ImageEvent = UiEvent<ImageData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ImageData {
    pub load_error: bool,
}
