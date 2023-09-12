use dioxus_core::Event;
use keyboard_types::Modifiers;

use crate::geometry::*;
use crate::prelude::{InteractionLocation, ModifiersInteraction};

pub type TouchEvent = Event<TouchData>;
pub struct TouchData {
    inner: Box<dyn HasTouchData>,
}

impl<E: HasTouchData> From<E> for TouchData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for TouchData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TouchData")
            .field("modifiers", &self.modifiers())
            .finish()
    }
}

impl PartialEq for TouchData {
    fn eq(&self, other: &Self) -> bool {
        self.modifiers() == other.modifiers()
    }
}

impl TouchData {
    /// Create a new TouchData
    pub fn new(inner: impl HasTouchData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Get the pointers that are currently down
    pub fn touches(&self) -> Vec<TouchPoint> {
        self.inner.touches()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl ModifiersInteraction for TouchData {
    fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of TouchData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedTouchData {
    alt_key: bool,
    ctrl_key: bool,
    meta_key: bool,
    shift_key: bool,
    touches: Vec<SerializedTouchPoint>,
}

#[cfg(feature = "serialize")]
impl From<&TouchData> for SerializedTouchData {
    fn from(data: &TouchData) -> Self {
        let modifiers = data.modifiers();
        Self {
            alt_key: modifiers.contains(Modifiers::ALT),
            ctrl_key: modifiers.contains(Modifiers::CONTROL),
            meta_key: modifiers.contains(Modifiers::META),
            shift_key: modifiers.contains(Modifiers::SHIFT),
            touches: data.touches().iter().map(|t| t.into()).collect(),
        }
    }
}

#[cfg(feature = "serialize")]
impl ModifiersInteraction for SerializedTouchData {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::default();
        if self.alt_key {
            modifiers.insert(Modifiers::ALT);
        }
        if self.ctrl_key {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.meta_key {
            modifiers.insert(Modifiers::META);
        }
        if self.shift_key {
            modifiers.insert(Modifiers::SHIFT);
        }
        modifiers
    }
}

#[cfg(feature = "serialize")]
impl HasTouchData for SerializedTouchData {
    fn touches(&self) -> Vec<TouchPoint> {
        Vec::new()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for TouchData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedTouchData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for TouchData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedTouchData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

pub trait HasTouchData: ModifiersInteraction + std::any::Any {
    /// Get the pointers that are currently down
    fn touches(&self) -> Vec<TouchPoint>;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct TouchPoint {
    inner: Box<dyn HasTouchPointData>,
}

impl<E: HasTouchPointData> From<E> for TouchPoint {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl TouchPoint {
    /// Create a new TouchPoint
    pub fn new(inner: impl HasTouchPointData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// A unique identifier for this touch point that will be the same for the duration of the touch
    fn identifier(&self) -> i32 {
        self.inner.identifier()
    }

    /// the pressure of the touch
    fn force(&self) -> f64 {
        self.inner.force()
    }

    /// the radius of the touch
    fn radius(&self) -> ScreenPoint {
        self.inner.radius()
    }

    /// the rotation of the touch in degrees between 0 and 90
    fn rotation(&self) -> f64 {
        self.inner.rotation()
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl InteractionLocation for TouchPoint {
    fn client_coordinates(&self) -> ClientPoint {
        self.inner.client_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.inner.page_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.inner.screen_coordinates()
    }
}

/// A trait for touch point data
pub trait HasTouchPointData: InteractionLocation + std::any::Any {
    /// A unique identifier for this touch point that will be the same for the duration of the touch
    fn identifier(&self) -> i32;

    /// the pressure of the touch
    fn force(&self) -> f64;

    /// the radius of the touch
    fn radius(&self) -> ScreenPoint;

    /// the rotation of the touch in degrees between 0 and 90
    fn rotation(&self) -> f64;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "serialize")]
/// A serialized version of TouchPoint
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
struct SerializedTouchPoint {
    identifier: i32,
    client_x: f64,
    client_y: f64,
    page_x: f64,
    page_y: f64,
    screen_x: f64,
    screen_y: f64,
    force: f64,
    radius_x: f64,
    radius_y: f64,
    rotation_angle: f64,
}

#[cfg(feature = "serialize")]
impl From<&TouchPoint> for SerializedTouchPoint {
    fn from(point: &TouchPoint) -> Self {
        let client_coordinates = point.client_coordinates();

        let page_coordinates = point.page_coordinates();
        let screen_coordinates = point.screen_coordinates();
        Self {
            identifier: point.identifier(),
            client_x: client_coordinates.x,
            client_y: client_coordinates.y,
            page_x: page_coordinates.x,
            page_y: page_coordinates.y,
            screen_x: screen_coordinates.x,
            screen_y: screen_coordinates.y,
            force: point.force(),
            radius_x: point.radius().x,
            radius_y: point.radius().y,
            rotation_angle: point.rotation(),
        }
    }
}

impl_event! {
    TouchData;
    /// touchstart
    ontouchstart

    /// touchmove
    ontouchmove

    /// touchend
    ontouchend

    /// touchcancel
    ontouchcancel
}
