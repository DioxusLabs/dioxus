use crate::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use crate::input_data::{MouseButton, MouseButtonSet};
use crate::prelude::*;
use dioxus_core::Event;
use keyboard_types::Modifiers;

pub type MouseEvent = Event<MouseData>;

/// A synthetic event that wraps a web-style [`MouseEvent`](https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent)
/// Data associated with a mouse event
pub struct MouseData {
    inner: Box<dyn HasMouseData>,
}

impl<E: HasMouseData + 'static> From<E> for MouseData {
    fn from(e: E) -> Self {
        Self { inner: Box::new(e) }
    }
}

impl std::fmt::Debug for MouseData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MouseData")
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .finish()
    }
}

impl<E: HasMouseData> PartialEq<E> for MouseData {
    fn eq(&self, other: &E) -> bool {
        self.coordinates() == other.coordinates()
            && self.modifiers() == other.modifiers()
            && self.held_buttons() == other.held_buttons()
            && self.trigger_button() == other.trigger_button()
    }
}

/// A trait for any object that has the data for a mouse event
pub trait HasMouseData: PointerInteraction {
    /// return self as Any
    fn as_any(&self) -> &dyn std::any::Any;
}

impl_event! {
    MouseData;

    /// Execute a callback when a button is clicked.
    ///
    /// ## Description
    ///
    /// An element receives a click event when a pointing device button (such as a mouse's primary mouse button)
    /// is both pressed and released while the pointer is located inside the element.
    ///
    /// - Bubbles: Yes
    /// - Cancelable: Yes
    /// - Interface(InteData): [`MouseEvent`]
    ///
    /// If the button is pressed on one element and the pointer is moved outside the element before the button
    /// is released, the event is fired on the most specific ancestor element that contained both elements.
    /// `click` fires after both the `mousedown` and `mouseup` events have fired, in that order.
    ///
    /// ## Example
    /// ```rust, ignore
    /// rsx!( button { "click me", onclick: move |_| tracing::info!("Clicked!`") } )
    /// ```
    ///
    /// ## Reference
    /// - <https://www.w3schools.com/tags/ev_onclick.asp>
    /// - <https://developer.mozilla.org/en-US/docs/Web/API/Element/click_event>
    onclick

    /// oncontextmenu
    oncontextmenu

    #[deprecated(since = "0.5.0", note = "use ondoubleclick instead")]
    ondblclick

    ondoubleclick: "ondblclick"

    /// onmousedown
    onmousedown

    /// onmouseenter
    onmouseenter

    /// onmouseleave
    onmouseleave

    /// onmousemove
    onmousemove

    /// onmouseout
    onmouseout

    /// onmouseover
    ///
    /// Triggered when the users's mouse hovers over an element.
    onmouseover

    /// onmouseup
    onmouseup
}

impl MouseData {
    /// Create a new instance of MouseData
    pub fn new(inner: impl HasMouseData + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// Downcast this event to a concrete event type
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }
}

impl InteractionLocation for MouseData {
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

impl InteractionElementOffset for MouseData {
    fn element_coordinates(&self) -> ElementPoint {
        self.inner.element_coordinates()
    }

    fn coordinates(&self) -> Coordinates {
        self.inner.coordinates()
    }
}

impl ModifiersInteraction for MouseData {
    /// The set of modifier keys which were pressed when the event occurred
    fn modifiers(&self) -> Modifiers {
        self.inner.modifiers()
    }
}

impl PointerInteraction for MouseData {
    /// The set of mouse buttons which were held when the event occurred.
    fn held_buttons(&self) -> MouseButtonSet {
        self.inner.held_buttons()
    }

    /// The mouse button that triggered the event
    ///
    // todo the following is kind of bad; should we just return None when the trigger_button is unreliable (and frankly irrelevant)? i guess we would need the event_type here
    /// This is only guaranteed to indicate which button was pressed during events caused by pressing or releasing a button. As such, it is not reliable for events such as mouseenter, mouseleave, mouseover, mouseout, or mousemove. For example, a value of MouseButton::Primary may also indicate that no button was pressed.
    fn trigger_button(&self) -> Option<MouseButton> {
        self.inner.trigger_button()
    }
}

impl PartialEq for MouseData {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates() == other.coordinates()
            && self.modifiers() == other.modifiers()
            && self.held_buttons() == other.held_buttons()
            && self.trigger_button() == other.trigger_button()
    }
}

#[cfg(feature = "serialize")]
/// A serialized version of [`MouseData`]
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
pub struct SerializedMouseData {
    /// Common data for all pointer/mouse events
    #[serde(flatten)]
    point_data: crate::point_interaction::SerializedPointInteraction,
}

#[cfg(feature = "serialize")]
impl SerializedMouseData {
    /// Create a new instance of SerializedMouseData
    pub fn new(
        trigger_button: Option<MouseButton>,
        held_buttons: MouseButtonSet,
        coordinates: Coordinates,
        modifiers: Modifiers,
    ) -> Self {
        Self {
            point_data: crate::point_interaction::SerializedPointInteraction::new(
                trigger_button,
                held_buttons,
                coordinates,
                modifiers,
            ),
        }
    }
}

#[cfg(feature = "serialize")]
impl From<&MouseData> for SerializedMouseData {
    fn from(e: &MouseData) -> Self {
        Self {
            point_data: crate::point_interaction::SerializedPointInteraction::from(e),
        }
    }
}

#[cfg(feature = "serialize")]
impl HasMouseData for SerializedMouseData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(feature = "serialize")]
impl InteractionLocation for SerializedMouseData {
    fn client_coordinates(&self) -> ClientPoint {
        self.point_data.client_coordinates()
    }

    fn page_coordinates(&self) -> PagePoint {
        self.point_data.page_coordinates()
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.point_data.screen_coordinates()
    }
}

#[cfg(feature = "serialize")]
impl InteractionElementOffset for SerializedMouseData {
    fn element_coordinates(&self) -> ElementPoint {
        self.point_data.element_coordinates()
    }
}

#[cfg(feature = "serialize")]
impl ModifiersInteraction for SerializedMouseData {
    fn modifiers(&self) -> Modifiers {
        self.point_data.modifiers()
    }
}

#[cfg(feature = "serialize")]
impl PointerInteraction for SerializedMouseData {
    fn held_buttons(&self) -> MouseButtonSet {
        self.point_data.held_buttons()
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.point_data.trigger_button()
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for MouseData {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SerializedMouseData::from(self).serialize(serializer)
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for MouseData {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = SerializedMouseData::deserialize(deserializer)?;
        Ok(Self {
            inner: Box::new(data),
        })
    }
}
