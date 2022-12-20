use dioxus_core::Event;

pub type SelectionEvent = Event<SelectionData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
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
