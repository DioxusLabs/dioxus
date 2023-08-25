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
    /// The event's coordinates relative to the application's viewport (as opposed to the coordinate within the page).
    ///
    /// For example, clicking in the top left corner of the viewport will always result in a mouse event with client coordinates (0., 0.), regardless of whether the page is scrolled horizontally.
    pub fn client_coordinates(&self) -> ClientPoint {
        self.inner.client_coordinates()
    }

    /// The event's coordinates relative to the padding edge of the target element
    ///
    /// For example, clicking in the top left corner of an element will result in element coordinates (0., 0.)
    pub fn element_coordinates(&self) -> ElementPoint {
        self.inner.element_coordinates()
    }

    /// The event's coordinates relative to the entire document. This includes any portion of the document not currently visible.
    ///
    /// For example, if the page is scrolled 200 pixels to the right and 300 pixels down, clicking in the top left corner of the viewport would result in page coordinates (200., 300.)
    pub fn page_coordinates(&self) -> PagePoint {
        self.inner.page_coordinates()
    }

    /// The event's coordinates relative to the entire screen. This takes into account the window's offset.
    pub fn screen_coordinates(&self) -> ScreenPoint {
        self.inner.screen_coordinates()
    }

    pub fn coordinates(&self) -> Coordinates {
        self.inner.coordinates()
    }

    /// The set of modifier keys which were pressed when the event occurred
    pub fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }

    /// The set of mouse buttons which were held when the event occurred.
    pub fn held_buttons(&self) -> MouseButtonSet {
        self.inner.held_buttons()
    }

    /// The mouse button that triggered the event
    ///
    // todo the following is kind of bad; should we just return None when the trigger_button is unreliable (and frankly irrelevant)? i guess we would need the event_type here
    /// This is only guaranteed to indicate which button was pressed during events caused by pressing or releasing a button. As such, it is not reliable for events such as mouseenter, mouseleave, mouseover, mouseout, or mousemove. For example, a value of MouseButton::Primary may also indicate that no button was pressed.
    pub fn trigger_button(&self) -> Option<MouseButton> {
        self.inner.trigger_button()
    }

    /// Downcast this event data to a specific type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

#[cfg(feature = "serialize")]
#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedDragData {
    mouse: crate::point_interaction::SerializedPointInteraction,
}

#[cfg(feature = "serialize")]
impl From<&DragData> for SerializedDragData {
    fn from(data: &DragData) -> Self {
        Self {
            mouse: crate::point_interaction::SerializedPointInteraction {
                client_coordinates: data.client_coordinates(),
                element_coordinates: data.element_coordinates(),
                page_coordinates: data.page_coordinates(),
                screen_coordinates: data.screen_coordinates(),
                modifiers: data.modifiers(),
                held_buttons: data.held_buttons(),
                trigger_button: data.trigger_button(),
            },
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
impl PointInteraction for SerializedDragData {
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
