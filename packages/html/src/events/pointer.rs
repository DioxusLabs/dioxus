use dioxus_core_types::Event;
use keyboard_types::Modifiers;

use crate::{geometry::*, input_data::*, prelude::*};

/// A synthetic event that wraps a web-style [`PointerEvent`](https://developer.mozilla.org/en-US/docs/Web/API/PointerEvent)
pub type PointerEvent = Event<PointerData>;

pub struct PointerData {
    inner: Box<dyn HasPointerData>,
}

impl PointerData {
    /// Create a new PointerData
    pub fn new(data: impl HasPointerData + 'static) -> Self {
        Self::from(data)
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl<E: HasPointerData + 'static> From<E> for PointerData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for PointerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PointerData")
            .field("pointer_id", &self.pointer_id())
            .field("width", &self.width())
            .field("height", &self.height())
            .field("pressure", &self.pressure())
            .field("tangential_pressure", &self.tangential_pressure())
            .field("tilt_x", &self.tilt_x())
            .field("tilt_y", &self.tilt_y())
            .field("twist", &self.twist())
            .field("pointer_type", &self.pointer_type())
            .field("is_primary", &self.is_primary())
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .finish()
    }
}

impl PartialEq for PointerData {
    fn eq(&self, other: &Self) -> bool {
        self.pointer_id() == other.pointer_id()
            && self.width() == other.width()
            && self.height() == other.height()
            && self.pressure() == other.pressure()
            && self.tangential_pressure() == other.tangential_pressure()
            && self.tilt_x() == other.tilt_x()
            && self.tilt_y() == other.tilt_y()
            && self.twist() == other.twist()
            && self.pointer_type() == other.pointer_type()
            && self.is_primary() == other.is_primary()
            && self.coordinates() == other.coordinates()
            && self.modifiers() == other.modifiers()
            && self.held_buttons() == other.held_buttons()
            && self.trigger_button() == other.trigger_button()
    }
}

/// A trait for any object that has the data for a pointer event
pub trait HasPointerData: PointerInteraction {
    /// Gets the unique identifier of the pointer causing the event.
    fn pointer_id(&self) -> i32;

    /// Gets the width (magnitude on the X axis), in CSS pixels, of the contact geometry of the pointer.
    fn width(&self) -> i32;

    /// Gets the height (magnitude on the Y axis), in CSS pixels, of the contact geometry of the pointer.
    fn height(&self) -> i32;

    /// Gets the normalized pressure of the pointer input in the range of 0 to 1,
    fn pressure(&self) -> f32;

    /// Gets the normalized tangential pressure of the pointer input (also known as barrel pressure or cylinder stress) in the range -1 to 1,
    fn tangential_pressure(&self) -> f32;

    /// Gets the plane angle (in degrees, in the range of -90 to 90) between the Y-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the Y axis.
    fn tilt_x(&self) -> i32;

    /// Gets the plane angle (in degrees, in the range of -90 to 90) between the X-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the X axis.
    fn tilt_y(&self) -> i32;

    /// Gets the clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.
    fn twist(&self) -> i32;

    /// Gets the device type that caused the event (mouse, pen, touch, etc.).
    fn pointer_type(&self) -> String;

    /// Gets if the pointer represents the primary pointer of this pointer type.
    fn is_primary(&self) -> bool;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
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
    /// Gets the unique identifier of the pointer causing the event.
    pub fn pointer_id(&self) -> i32 {
        self.inner.pointer_id()
    }

    /// Gets the width (magnitude on the X axis), in CSS pixels, of the contact geometry of the pointer.
    pub fn width(&self) -> i32 {
        self.inner.width()
    }

    /// Gets the height (magnitude on the Y axis), in CSS pixels, of the contact geometry of the pointer.
    pub fn height(&self) -> i32 {
        self.inner.height()
    }

    /// Gets the normalized pressure of the pointer input in the range of 0 to 1,
    pub fn pressure(&self) -> f32 {
        self.inner.pressure()
    }

    /// Gets the normalized tangential pressure of the pointer input (also known as barrel pressure or cylinder stress) in the range -1 to 1,
    pub fn tangential_pressure(&self) -> f32 {
        self.inner.tangential_pressure()
    }

    /// Gets the plane angle (in degrees, in the range of -90 to 90) between the Y-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the Y axis.
    pub fn tilt_x(&self) -> i32 {
        self.inner.tilt_x()
    }

    /// Gets the plane angle (in degrees, in the range of -90 to 90) between the X-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the X axis.
    pub fn tilt_y(&self) -> i32 {
        self.inner.tilt_y()
    }

    /// Gets the clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.
    pub fn twist(&self) -> i32 {
        self.inner.twist()
    }

    /// Gets the device type that caused the event (mouse, pen, touch, etc.).
    pub fn pointer_type(&self) -> String {
        self.inner.pointer_type()
    }

    /// Gets if the pointer represents the primary pointer of this pointer type.
    pub fn is_primary(&self) -> bool {
        self.inner.is_primary()
    }
}

impl InteractionLocation for PointerData {
    fn client_coordinates(&self) -> ClientPoint {
        self.inner.client_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.inner.screen_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.inner.page_coordinates()
    }
}

impl InteractionElementOffset for PointerData {
    fn element_coordinates(&self) -> ElementPoint {
        self.inner.element_coordinates()
    }
}

impl ModifiersInteraction for PointerData {
    fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }
}

impl PointerInteraction for PointerData {
    fn held_buttons(&self) -> MouseButtonSet {
        self.inner.held_buttons()
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.inner.trigger_button()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of PointerData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedPointerData {
    /// Common data for all pointer/mouse events
    #[serde(flatten)]
    point_data: crate::point_interaction::SerializedPointInteraction,

    /// The unique identifier of the pointer causing the event.
    pointer_id: i32,

    /// The width (magnitude on the X axis), in CSS pixels, of the contact geometry of the pointer.
    width: i32,

    /// The height (magnitude on the Y axis), in CSS pixels, of the contact geometry of the pointer.
    height: i32,

    /// The normalized pressure of the pointer input in the range of 0 to 1,
    pressure: f32,

    /// The normalized tangential pressure of the pointer input (also known as barrel pressure or cylinder stress) in the range -1 to 1,
    tangential_pressure: f32,

    /// The plane angle (in degrees, in the range of -90 to 90) between the Y-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the Y axis.
    tilt_x: i32,

    /// The plane angle (in degrees, in the range of -90 to 90) between the X-Z plane and the plane containing both the transducer (e.g. pen stylus) axis and the X axis.
    tilt_y: i32,

    /// The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.The clockwise rotation of the pointer (e.g. pen stylus) around its major axis in degrees, with a value in the range 0 to 359.
    twist: i32,

    /// Indicates the device type that caused the event (mouse, pen, touch, etc.).
    pointer_type: String,

    /// Indicates if the pointer represents the primary pointer of this pointer type.
    is_primary: bool,
}

#[cfg(feature = "serialize")]
impl HasPointerData for SerializedPointerData {
    fn pointer_id(&self) -> i32 {
        self.pointer_id
    }

    fn width(&self) -> i32 {
        self.width
    }

    fn height(&self) -> i32 {
        self.height
    }

    fn pressure(&self) -> f32 {
        self.pressure
    }

    fn tangential_pressure(&self) -> f32 {
        self.tangential_pressure
    }

    fn tilt_x(&self) -> i32 {
        self.tilt_x
    }

    fn tilt_y(&self) -> i32 {
        self.tilt_y
    }

    fn twist(&self) -> i32 {
        self.twist
    }

    fn pointer_type(&self) -> String {
        self.pointer_type.clone()
    }

    fn is_primary(&self) -> bool {
        self.is_primary
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl InteractionLocation for SerializedPointerData {
    fn client_coordinates(&self) -> ClientPoint {
        self.point_data.client_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.point_data.screen_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.point_data.page_coordinates()
    }
}

#[cfg(feature = "serialize")]
impl InteractionElementOffset for SerializedPointerData {
    fn element_coordinates(&self) -> ElementPoint {
        self.point_data.element_coordinates()
    }
}

#[cfg(feature = "serialize")]
impl ModifiersInteraction for SerializedPointerData {
    fn modifiers(&self) -> Modifiers {
        self.point_data.modifiers()
    }
}

#[cfg(feature = "serialize")]
impl PointerInteraction for SerializedPointerData {
    fn held_buttons(&self) -> MouseButtonSet {
        self.point_data.held_buttons()
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.point_data.trigger_button()
    }
}

#[cfg(feature = "serialize")]
impl From<&PointerData> for SerializedPointerData {
    fn from(data: &PointerData) -> Self {
        Self {
            point_data: data.into(),
            pointer_id: data.pointer_id(),
            width: data.width(),
            height: data.height(),
            pressure: data.pressure(),
            tangential_pressure: data.tangential_pressure(),
            tilt_x: data.tilt_x(),
            tilt_y: data.tilt_y(),
            twist: data.twist(),
            pointer_type: data.pointer_type().to_string(),
            is_primary: data.is_primary(),
        }
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for PointerData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedPointerData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for PointerData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedPointerData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}
