use crate::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use crate::input_data::{MouseButton, MouseButtonSet};
use crate::prelude::*;

use dioxus_core_types::Event;
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

impl<E: HasDragData + 'static> From<E> for DragData {
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
    /// Create a new DragData
    pub fn new(inner: impl HasDragData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event data to a specific type
    #[inline(always)]
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        HasDragData::as_any(&*self.inner).downcast_ref::<T>()
    }
}

impl crate::HasFileData for DragData {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn crate::file_data::FileEngine>> {
        self.inner.files()
    }
}

impl InteractionLocation for DragData {
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

impl InteractionElementOffset for DragData {
    fn element_coordinates(&self) -> ElementPoint {
        self.inner.element_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.inner.coordinates()
    }
}

impl ModifiersInteraction for DragData {
    fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }
}

impl PointerInteraction for DragData {
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
    pub mouse: crate::point_interaction::SerializedPointInteraction,

    #[cfg(feature = "file-engine")]
    #[serde(default)]
    files: Option<crate::file_data::SerializedFileEngine>,
}

#[cfg(feature = "serialize")]
impl SerializedDragData {
    fn new(drag: &DragData) -> Self {
        Self {
            mouse: crate::point_interaction::SerializedPointInteraction::from(drag),
            #[cfg(feature = "file-engine")]
            files: None,
        }
    }
}

#[cfg(feature = "serialize")]
impl HasDragData for SerializedDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl crate::file_data::HasFileData for SerializedDragData {
    #[cfg(feature = "file-engine")]
    fn files(&self) -> Option<std::sync::Arc<dyn crate::file_data::FileEngine>> {
        self.files
            .as_ref()
            .map(|files| std::sync::Arc::new(files.clone()) as _)
    }
}

#[cfg(feature = "serialize")]
impl HasMouseData for SerializedDragData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl InteractionLocation for SerializedDragData {
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
impl InteractionElementOffset for SerializedDragData {
    fn element_coordinates(&self) -> ElementPoint {
        self.mouse.element_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.mouse.coordinates()
    }
}

#[cfg(feature = "serialize")]
impl ModifiersInteraction for SerializedDragData {
    fn modifiers(&self) -> Modifiers {
        self.mouse.modifiers()
    }
}

#[cfg(feature = "serialize")]
impl PointerInteraction for SerializedDragData {
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
        SerializedDragData::new(self).serialize(serializer)
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
pub trait HasDragData: HasMouseData + crate::HasFileData {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

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
