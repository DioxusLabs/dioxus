use dioxus_core::UiEvent;

pub type FocusEvent = UiEvent<FocusData>;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct FocusData {/* DOMEventInner:  Send + SyncTarget relatedTarget */}

impl_event! [
    FocusData;

    /// onfocus
    onfocus

    // onfocusout
    onfocusout

    // onfocusin
    onfocusin

    /// onblur
    onblur
];
