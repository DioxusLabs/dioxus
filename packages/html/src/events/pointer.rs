use super::make_listener;
use dioxus_core::{Listener, NodeFactory};

event! {
    PointerEvent: [
        /// pointerdown
        onpointerdown

        /// pointermove
        onpointermove

        /// pointerup
        onpointerup

        /// pointercancel
        onpointercancel

        /// gotpointercapture
        ongotpointercapture

        /// lostpointercapture
        onlostpointercapture

        /// pointerenter
        onpointerenter

        /// pointerleave
        onpointerleave

        /// pointerover
        onpointerover

        /// pointerout
        onpointerout
    ];
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct PointerEvent {
    // Mouse only
    pub alt_key: bool,
    pub button: i16,
    pub buttons: u16,
    pub client_x: i32,
    pub client_y: i32,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub page_x: i32,
    pub page_y: i32,
    pub screen_x: i32,
    pub screen_y: i32,
    pub shift_key: bool,
    pub pointer_id: i32,
    pub width: i32,
    pub height: i32,
    pub pressure: f32,
    pub tangential_pressure: f32,
    pub tilt_x: i32,
    pub tilt_y: i32,
    pub twist: i32,
    pub pointer_type: String,
    pub is_primary: bool,
    // pub get_modifier_state: bool,
}
