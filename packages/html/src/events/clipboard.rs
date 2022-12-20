use dioxus_core::Event;

pub type ClipboardEvent = Event<ClipboardData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardData {
    // DOMDataTransfer clipboardData
}

impl_event![
    ClipboardData;

    /// oncopy
    oncopy

    /// oncut
    oncut

    /// onpaste
    onpaste
];
