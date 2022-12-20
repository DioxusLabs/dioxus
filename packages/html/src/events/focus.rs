use dioxus_core::Event;

pub type FocusEvent = Event<FocusData>;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
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
