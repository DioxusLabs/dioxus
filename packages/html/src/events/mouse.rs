use crate::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use crate::input_data::{
    decode_mouse_button_set, encode_mouse_button_set, MouseButton, MouseButtonSet,
};
use dioxus_core::Event;
use keyboard_types::Modifiers;
use std::fmt::{Debug, Formatter};

pub type MouseEvent = Event<MouseData>;

/// A synthetic event that wraps a web-style [`MouseEvent`](https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent)
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Default, PartialEq, Eq)]
/// Data associated with a mouse event
///
/// Do not use the deprecated fields; they may change or become private in the future.
pub struct MouseData {
    /// True if the alt key was down when the mouse event was fired.
    #[deprecated(since = "0.3.0", note = "use modifiers() instead")]
    pub alt_key: bool,

    /// The button number that was pressed (if applicable) when the mouse event was fired.
    #[deprecated(since = "0.3.0", note = "use trigger_button() instead")]
    pub button: i16,

    /// Indicates which buttons are pressed on the mouse (or other input device) when a mouse event is triggered.
    ///
    /// Each button that can be pressed is represented by a given number (see below). If more than one button is pressed, the button values are added together to produce a new number. For example, if the secondary (2) and auxiliary (4) buttons are pressed simultaneously, the value is 6 (i.e., 2 + 4).
    ///
    /// - 1: Primary button (usually the left button)
    /// - 2: Secondary button (usually the right button)
    /// - 4: Auxiliary button (usually the mouse wheel button or middle button)
    /// - 8: 4th button (typically the "Browser Back" button)
    /// - 16 : 5th button (typically the "Browser Forward" button)
    #[deprecated(since = "0.3.0", note = "use held_buttons() instead")]
    pub buttons: u16,

    /// The horizontal coordinate within the application's viewport at which the event occurred (as opposed to the coordinate within the page).
    ///
    /// For example, clicking on the left edge of the viewport will always result in a mouse event with a clientX value of 0, regardless of whether the page is scrolled horizontally.
    #[deprecated(since = "0.3.0", note = "use client_coordinates() instead")]
    pub client_x: i32,

    /// The vertical coordinate within the application's viewport at which the event occurred (as opposed to the coordinate within the page).
    ///
    /// For example, clicking on the top edge of the viewport will always result in a mouse event with a clientY value of 0, regardless of whether the page is scrolled vertically.
    #[deprecated(since = "0.3.0", note = "use client_coordinates() instead")]
    pub client_y: i32,

    /// True if the control key was down when the mouse event was fired.
    #[deprecated(since = "0.3.0", note = "use modifiers() instead")]
    pub ctrl_key: bool,

    /// True if the meta key was down when the mouse event was fired.
    #[deprecated(since = "0.3.0", note = "use modifiers() instead")]
    pub meta_key: bool,

    /// The offset in the X coordinate of the mouse pointer between that event and the padding edge of the target node.
    #[deprecated(since = "0.3.0", note = "use element_coordinates() instead")]
    pub offset_x: i32,

    /// The offset in the Y coordinate of the mouse pointer between that event and the padding edge of the target node.
    #[deprecated(since = "0.3.0", note = "use element_coordinates() instead")]
    pub offset_y: i32,

    /// The X (horizontal) coordinate (in pixels) of the mouse, relative to the left edge of the entire document. This includes any portion of the document not currently visible.
    ///
    /// Being based on the edge of the document as it is, this property takes into account any horizontal scrolling of the page. For example, if the page is scrolled such that 200 pixels of the left side of the document are scrolled out of view, and the mouse is clicked 100 pixels inward from the left edge of the view, the value returned by pageX will be 300.
    #[deprecated(since = "0.3.0", note = "use page_coordinates() instead")]
    pub page_x: i32,

    /// The Y (vertical) coordinate in pixels of the event relative to the whole document.
    ///
    /// See `page_x`.
    #[deprecated(since = "0.3.0", note = "use page_coordinates() instead")]
    pub page_y: i32,

    /// The X coordinate of the mouse pointer in global (screen) coordinates.
    #[deprecated(since = "0.3.0", note = "use screen_coordinates() instead")]
    pub screen_x: i32,

    /// The Y coordinate of the mouse pointer in global (screen) coordinates.
    #[deprecated(since = "0.3.0", note = "use screen_coordinates() instead")]
    pub screen_y: i32,

    /// True if the shift key was down when the mouse event was fired.
    #[deprecated(since = "0.3.0", note = "use modifiers() instead")]
    pub shift_key: bool,
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

    /// ondoubleclick
    ondoubleclick

    /// ondoubleclick
    ondblclick

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
    /// Construct MouseData with the specified properties
    ///
    /// Note: the current implementation truncates coordinates. In the future, when we change the internal representation, it may also support a fractional part.
    pub fn new(
        coordinates: Coordinates,
        trigger_button: Option<MouseButton>,
        held_buttons: MouseButtonSet,
        modifiers: Modifiers,
    ) -> Self {
        let alt_key = modifiers.contains(Modifiers::ALT);
        let ctrl_key = modifiers.contains(Modifiers::CONTROL);
        let meta_key = modifiers.contains(Modifiers::META);
        let shift_key = modifiers.contains(Modifiers::SHIFT);

        let [client_x, client_y]: [i32; 2] = coordinates.client().cast().into();
        let [offset_x, offset_y]: [i32; 2] = coordinates.element().cast().into();
        let [page_x, page_y]: [i32; 2] = coordinates.page().cast().into();
        let [screen_x, screen_y]: [i32; 2] = coordinates.screen().cast().into();

        #[allow(deprecated)]
        Self {
            alt_key,
            ctrl_key,
            meta_key,
            shift_key,

            button: trigger_button.map_or(0, |b| b.into_web_code()),
            buttons: encode_mouse_button_set(held_buttons),

            client_x,
            client_y,
            offset_x,
            offset_y,
            page_x,
            page_y,
            screen_x,
            screen_y,
        }
    }

    /// The event's coordinates relative to the application's viewport (as opposed to the coordinate within the page).
    ///
    /// For example, clicking in the top left corner of the viewport will always result in a mouse event with client coordinates (0., 0.), regardless of whether the page is scrolled horizontally.
    pub fn client_coordinates(&self) -> ClientPoint {
        #[allow(deprecated)]
        ClientPoint::new(self.client_x.into(), self.client_y.into())
    }

    /// The event's coordinates relative to the padding edge of the target element
    ///
    /// For example, clicking in the top left corner of an element will result in element coordinates (0., 0.)
    pub fn element_coordinates(&self) -> ElementPoint {
        #[allow(deprecated)]
        ElementPoint::new(self.offset_x.into(), self.offset_y.into())
    }

    /// The event's coordinates relative to the entire document. This includes any portion of the document not currently visible.
    ///
    /// For example, if the page is scrolled 200 pixels to the right and 300 pixels down, clicking in the top left corner of the viewport would result in page coordinates (200., 300.)
    pub fn page_coordinates(&self) -> PagePoint {
        #[allow(deprecated)]
        PagePoint::new(self.page_x.into(), self.page_y.into())
    }

    /// The event's coordinates relative to the entire screen. This takes into account the window's offset.
    pub fn screen_coordinates(&self) -> ScreenPoint {
        #[allow(deprecated)]
        ScreenPoint::new(self.screen_x.into(), self.screen_y.into())
    }

    pub fn coordinates(&self) -> Coordinates {
        Coordinates::new(
            self.screen_coordinates(),
            self.client_coordinates(),
            self.element_coordinates(),
            self.page_coordinates(),
        )
    }

    /// The set of modifier keys which were pressed when the event occurred
    pub fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

        #[allow(deprecated)]
        {
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
        }

        modifiers
    }

    /// The set of mouse buttons which were held when the event occurred.
    pub fn held_buttons(&self) -> MouseButtonSet {
        #[allow(deprecated)]
        decode_mouse_button_set(self.buttons)
    }

    /// The mouse button that triggered the event
    ///
    // todo the following is kind of bad; should we just return None when the trigger_button is unreliable (and frankly irrelevant)? i guess we would need the event_type here
    /// This is only guaranteed to indicate which button was pressed during events caused by pressing or releasing a button. As such, it is not reliable for events such as mouseenter, mouseleave, mouseover, mouseout, or mousemove. For example, a value of MouseButton::Primary may also indicate that no button was pressed.
    pub fn trigger_button(&self) -> Option<MouseButton> {
        #[allow(deprecated)]
        Some(MouseButton::from_web_code(self.button))
    }
}

impl Debug for MouseData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MouseData")
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .finish()
    }
}
