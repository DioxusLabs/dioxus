use dioxus_core_types::Event;
use std::fmt::Formatter;

use crate::geometry::*;
use crate::input_data::{MouseButton, MouseButtonSet};
use crate::prelude::*;

use super::HasMouseData;

/// A synthetic event that wraps a web-style
/// [`WheelEvent`](https://developer.mozilla.org/en-US/docs/Web/API/WheelEvent)
pub type WheelEvent = Event<WheelData>;

/// Data associated with a [WheelEvent]
pub struct WheelData {
    inner: Box<dyn HasWheelData>,
}

impl<E: HasWheelData> From<E> for WheelData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for WheelData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WheelData")
            .field("delta", &self.delta())
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .finish()
    }
}

impl PartialEq for WheelData {
    fn eq(&self, other: &Self) -> bool {
        self.inner.delta() == other.inner.delta()
    }
}

impl WheelData {
    pub fn new(inner: impl HasWheelData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The amount of wheel movement
    #[allow(deprecated)]
    pub fn delta(&self) -> WheelDelta {
        self.inner.delta()
    }

    /// Downcast this event to a concrete event type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        HasWheelData::as_any(&*self.inner).downcast_ref::<T>()
    }
}

impl InteractionLocation for WheelData {
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

impl InteractionElementOffset for WheelData {
    fn element_coordinates(&self) -> ElementPoint {
        self.inner.element_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.inner.coordinates()
    }
}

impl ModifiersInteraction for WheelData {
    fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }
}

impl PointerInteraction for WheelData {
    fn held_buttons(&self) -> MouseButtonSet {
        self.inner.held_buttons()
    }

    // todo the following is kind of bad; should we just return None when the trigger_button is unreliable (and frankly irrelevant)? i guess we would need the event_type here
    fn trigger_button(&self) -> Option<MouseButton> {
        self.inner.trigger_button()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of WheelData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedWheelData {
    #[serde(flatten)]
    pub mouse: crate::point_interaction::SerializedPointInteraction,

    pub delta_mode: u32,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
}

#[cfg(feature = "serialize")]
impl SerializedWheelData {
    /// Create a new SerializedWheelData
    pub fn new(wheel: &WheelData) -> Self {
        let delta_mode = match wheel.delta() {
            WheelDelta::Pixels(_) => 0,
            WheelDelta::Lines(_) => 1,
            WheelDelta::Pages(_) => 2,
        };
        let delta_raw = wheel.delta().strip_units();
        Self {
            mouse: crate::point_interaction::SerializedPointInteraction::from(wheel),
            delta_mode,
            delta_x: delta_raw.x,
            delta_y: delta_raw.y,
            delta_z: delta_raw.z,
        }
    }
}

#[cfg(feature = "serialize")]
impl From<&WheelData> for SerializedWheelData {
    fn from(data: &WheelData) -> Self {
        Self::new(data)
    }
}

#[cfg(feature = "serialize")]
impl HasWheelData for SerializedWheelData {
    fn delta(&self) -> WheelDelta {
        WheelDelta::from_web_attributes(self.delta_mode, self.delta_x, self.delta_y, self.delta_z)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl HasMouseData for SerializedWheelData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl InteractionLocation for SerializedWheelData {
    fn client_coordinates(&self) -> ClientPoint {
        self.mouse.client_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.mouse.page_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.mouse.screen_coordinates()
    }
}

#[cfg(feature = "serialize")]
impl InteractionElementOffset for SerializedWheelData {
    fn element_coordinates(&self) -> ElementPoint {
        self.mouse.element_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.mouse.coordinates()
    }
}

#[cfg(feature = "serialize")]
impl ModifiersInteraction for SerializedWheelData {
    fn modifiers(&self) -> Modifiers {
        self.mouse.modifiers()
    }
}

#[cfg(feature = "serialize")]
impl PointerInteraction for SerializedWheelData {
    fn held_buttons(&self) -> MouseButtonSet {
        self.mouse.held_buttons()
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.mouse.trigger_button()
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for WheelData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedWheelData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for WheelData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedWheelData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

impl_event![
    WheelData;

    /// Called when the mouse wheel is rotated over an element.
    onwheel
];

pub trait HasWheelData: HasMouseData + std::any::Any {
    /// The amount of wheel movement
    fn delta(&self) -> WheelDelta;

    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}
