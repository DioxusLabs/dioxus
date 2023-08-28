use crate::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use crate::input_data::{MouseButton, MouseButtonSet};
use crate::prelude::PointInteraction;

use dioxus_core::Event;
use keyboard_types::Modifiers;

use crate::HasMouseData;

pub type DragEvent = Event<DragData>;

/// The DragEvent interface is a DOM event that represents a drag and drop interaction. The user initiates a drag by
/// placing a pointer device (such as a mouse) on the touch surface and then dragging the pointer to a new location
/// (such as another DOM element). Applications are free to interpret a drag and drop interaction in an
/// application-specific way.
pub struct DragData {
    inner: Box<dyn HasDragData>,
}

impl<E: HasDragData> From<E> for DragData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for DragData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragData")
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .finish()
    }
}

impl PartialEq for DragData {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates() == other.coordinates()
            && self.modifiers() == other.modifiers()
            && self.held_buttons() == other.held_buttons()
            && self.trigger_button() == other.trigger_button()
    }
}

impl DragData {
    /// Downcast this event data to a specific type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl PointInteraction for DragData {
    fn client_coordinates(&self) -> ClientPoint {
        self.inner.client_coordinates()
    }

    fn element_coordinates(&self) -> ElementPoint {
        self.inner.element_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.inner.page_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.inner.screen_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.inner.coordinates()
    }

    fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.inner.held_buttons()
    }

    // todo the following is kind of bad; should we just return None when the trigger_button is unreliable (and frankly irrelevant)? i guess we would need the event_type here
    fn trigger_button(&self) -> Option<MouseButton> {
        self.inner.trigger_button()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of DragData
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
pub struct SerializedDragData {
    mouse: crate::point_interaction::SerializedPointInteraction,
}

#[cfg(feature = "serialize")]
impl From<&DragData> for SerializedDragData {
    fn from(data: &DragData) -> Self {
        Self {
            mouse: crate::point_interaction::SerializedPointInteraction::from(data),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasDragData for SerializedDragData {}

#[cfg(feature = "serialize")]
impl HasMouseData for SerializedDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl crate::prelude::PointInteraction for SerializedDragData {
    fn client_coordinates(&self) -> ClientPoint {
        self.mouse.client_coordinates()
    }

    fn element_coordinates(&self) -> ElementPoint {
        self.mouse.element_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.mouse.page_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.mouse.screen_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.mouse.coordinates()
    }

    fn modifiers(&self) -> Modifiers {
        self.mouse.modifiers()
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.mouse.held_buttons()
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.mouse.trigger_button()
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for DragData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedDragData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for DragData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedDragData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}

/// A trait for any object that has the data for a drag event
pub trait HasDragData: HasMouseData {}

impl_event! {
    DragData;

    /// ondrag
    ondrag

    /// ondragend
    ondragend

    /// ondragenter
    ondragenter

    /// ondragexit
    ondragexit

    /// ondragleave
    ondragleave

    /// ondragover
    ondragover

    /// ondragstart
    ondragstart

    /// ondrop
    ondrop
}
