use std::fmt::{Debug, Formatter};

use dioxus_core::Event;

use crate::point_interaction::{PointData, PointInteraction};

pub type PointerEvent = Event<PointerData>;
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq)]
pub struct PointerData {
    /// Common data for all pointer/mouse events
    #[cfg_attr(feature = "serialize", serde(flatten))]
    point_data: PointData,
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
}

impl_event![
    PointerData;
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

impl PointInteraction for PointerData {
    fn get_point_data(&self) -> PointData {
        self.point_data
    }
}

impl PointerData {
    pub fn new(
        point_data: PointData,
        pointer_id: i32,
        width: i32,
        height: i32,
        pressure: f32,
        tangential_pressure: f32,
        tilt_x: i32,
        tilt_y: i32,
        twist: i32,
        pointer_type: String,
        is_primary: bool,
    ) -> Self {
        Self {
            point_data,
            pointer_id,
            width,
            height,
            pressure,
            tangential_pressure,
            tilt_x,
            tilt_y,
            twist,
            pointer_type,
            is_primary,
        }
    }
}

impl Debug for PointerData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PointerData")
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .field("pointer_id", &self.pointer_id)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pressure", &self.pressure)
            .field("tangential_pressure", &self.tangential_pressure)
            .field("tilt_x", &self.tilt_x)
            .field("tilt_y", &self.tilt_y)
            .field("twist", &self.twist)
            .field("pointer_type", &self.pointer_type)
            .field("is_primary", &self.is_primary)
            .finish()
    }
}
