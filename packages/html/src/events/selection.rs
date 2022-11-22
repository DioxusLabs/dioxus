use dioxus_core::UiEvent;

pub type SelectionEvent = UiEvent<SelectionData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SelectionData {}

impl_event! [
    SelectionData;

    /// select
    onselect

    /// selectstart
    onselectstart

    /// selectionchange
    onselectionchange
];
