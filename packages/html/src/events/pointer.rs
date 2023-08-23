use std::fmt::Debug;

use dioxus_core::Event;

use crate::point_interaction::{PointData, PointInteraction};

/// A synthetic event that wraps a web-style [`PointerEvent`](https://developer.mozilla.org/en-US/docs/Web/API/PointerEvent)
pub type PointerEvent = Event<PointerData>;

/// Data associated with a pointer event, aside from the data shared with mouse events
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PointerEventData {
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

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PointerData {
    /// Common data for all pointer/mouse events
    #[cfg_attr(feature = "serialize", serde(flatten))]
    point_data: PointData,

    /// The unique identifier of the pointer causing the event.
    #[deprecated(since = "0.5.0", note = "use pointer_id() instead")]
    pub pointer_id: i32,

    /// The width (magnitude on the X axis), in CSS pixels, of the contact geometry of the pointer.
    #[deprecated(since = "0.5.0", note = "use width() instead")]
    pub width: i32,

    /// The height (magnitude on the Y axis), in CSS pixels, of the contact geometry of the pointer.
    #[deprecated(since = "0.5.0", note = "use height() instead")]
    pub height: i32,

    /// The normalized pressure of the pointer input in the range of 0 to 1,
    #[deprecated(since = "0.5.0", note = "use pressure() instead")]
    pub pressure: f32,

    /// The normalized tangential pressure of the pointer input (also known as barrel pressure or cylinder stress) in the range -1 to 1,
    #[deprecated(since = "0.5.0", note = "use tangential_pressure() instead")]
    pub tangential_pressure: f32,

    /// The plane angle (in degrees, in the range of -90 to 90) between the Y-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the Y axis.
    #[deprecated(since = "0.5.0", note = "use tilt_x() instead")]
    pub tilt_x: i32,

    /// The plane angle (in degrees, in the range of -90 to 90) between the X-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the X axis.
    #[deprecated(since = "0.5.0", note = "use tilt_y() instead")]
    pub tilt_y: i32,

    /// The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.
    #[deprecated(since = "0.5.0", note = "use twist() instead")]
    pub twist: i32,

    /// Indicates the device type that caused the event (mouse, pen, touch, etc.).
    #[deprecated(since = "0.5.0", note = "use pointer_type() instead")]
    pub pointer_type: String,

    /// Indicates if the pointer represents the primary pointer of this pointer type.
    #[deprecated(since = "0.5.0", note = "use is_primary() instead")]
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

impl PointerData {
    pub fn new(point_data: PointData, pointer_event_data: PointerEventData) -> Self {
        #[allow(deprecated)]
        Self {
            point_data,
            pointer_id: pointer_event_data.pointer_id,
            width: pointer_event_data.width,
            height: pointer_event_data.height,
            pressure: pointer_event_data.pressure,
            tangential_pressure: pointer_event_data.tangential_pressure,
            tilt_x: pointer_event_data.tilt_x,
            tilt_y: pointer_event_data.tilt_y,
            twist: pointer_event_data.twist,
            pointer_type: pointer_event_data.pointer_type,
            is_primary: pointer_event_data.is_primary,
        }
    }

    /// Gets the unique identifier of the pointer causing the event.
    pub fn pointer_id(&self) -> i32 {
        #[allow(deprecated)]
        self.pointer_id
    }

    /// Gets the width (magnitude on the X axis), in CSS pixels, of the contact geometry of the pointer.
    pub fn width(&self) -> i32 {
        #[allow(deprecated)]
        self.width
    }

    /// Gets the height (magnitude on the Y axis), in CSS pixels, of the contact geometry of the pointer.
    pub fn height(&self) -> i32 {
        #[allow(deprecated)]
        self.height
    }

    /// Gets the normalized pressure of the pointer input in the range of 0 to 1,
    pub fn pressure(&self) -> f32 {
        #[allow(deprecated)]
        self.pressure
    }

    /// Gets the normalized tangential pressure of the pointer input (also known as barrel pressure or cylinder stress) in the range -1 to 1,
    pub fn tangential_pressure(&self) -> f32 {
        #[allow(deprecated)]
        self.tangential_pressure
    }

    /// Gets the plane angle (in degrees, in the range of -90 to 90) between the Y-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the Y axis.
    pub fn tilt_x(&self) -> i32 {
        #[allow(deprecated)]
        self.tilt_x
    }

    /// Gets the plane angle (in degrees, in the range of -90 to 90) between the X-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the X axis.
    pub fn tilt_y(&self) -> i32 {
        #[allow(deprecated)]
        self.tilt_y
    }

    /// Gets the clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.
    pub fn twist(&self) -> i32 {
        #[allow(deprecated)]
        self.twist
    }

    /// Gets the device type that caused the event (mouse, pen, touch, etc.).
    pub fn pointer_type(&self) -> &str {
        #[allow(deprecated)]
        self.pointer_type.as_str()
    }

    /// Gets if the pointer represents the primary pointer of this pointer type.
    pub fn is_primary(&self) -> bool {
        #[allow(deprecated)]
        self.is_primary
    }
}

impl PointInteraction for PointerData {
    fn get_point_data(&self) -> PointData {
        self.point_data
    }
}
