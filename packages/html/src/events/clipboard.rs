use dioxus_core::UiEvent;

pub type ClipboardEvent = UiEvent<ClipboardData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ClipboardData {
    // DOMDataTransfer clipboardData
}
