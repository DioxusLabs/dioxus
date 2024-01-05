use keyboard_types::Modifiers;

use crate::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
};

/// A interaction that contains data about the location of the event.
pub trait InteractionLocation {
    /// Gets the coordinates of the event relative to the browser viewport.
    fn client_coordinates(&self) -> ClientPoint;

    /// Gets the coordinates of the event relative to the screen.
    fn screen_coordinates(&self) -> ScreenPoint;

    /// Gets the coordinates of the event relative to the page.
    fn page_coordinates(&self) -> PagePoint;
}

/// A interaction that contains data about the location of the event.
pub trait InteractionElementOffset: InteractionLocation {
    /// Gets the coordinates of the event.
    fn coordinates(&self) -> Coordinates {
        Coordinates::new(
            self.screen_coordinates(),
            self.client_coordinates(),
            self.element_coordinates(),
            self.page_coordinates(),
        )
    }

    /// Gets the coordinates of the event relative to the target element.
    fn element_coordinates(&self) -> ElementPoint;
}

/// A interaction that contains data about the pointer button(s) that triggered the event.
pub trait PointerInteraction: InteractionElementOffset + ModifiersInteraction {
    /// Gets the button that triggered the event.
    fn trigger_button(&self) -> Option<MouseButton>;

    /// Gets the buttons that are currently held down.
    fn held_buttons(&self) -> MouseButtonSet;
}

/// A interaction that contains data about the current state of the keyboard modifiers.
pub trait ModifiersInteraction {
    /// Gets the modifiers of the pointer event.
    fn modifiers(&self) -> Modifiers;
}

#[cfg(feature = "serialize")]
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
pub(crate) struct SerializedPointInteraction {
    pub alt_key: bool,

    /// The button number that was pressed (if applicable) when the mouse event was fired.
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
    pub buttons: u16,

    /// The horizontal coordinate within the application's viewport at which the event occurred (as opposed to the coordinate within the page).
    ///
    /// For example, clicking on the left edge of the viewport will always result in a mouse event with a clientX value of 0, regardless of whether the page is scrolled horizontally.
    pub client_x: i32,

    /// The vertical coordinate within the application's viewport at which the event occurred (as opposed to the coordinate within the page).
    ///
    /// For example, clicking on the top edge of the viewport will always result in a mouse event with a clientY value of 0, regardless of whether the page is scrolled vertically.
    pub client_y: i32,

    /// True if the control key was down when the mouse event was fired.
    pub ctrl_key: bool,

    /// True if the meta key was down when the mouse event was fired.
    pub meta_key: bool,

    /// The offset in the X coordinate of the mouse pointer between that event and the padding edge of the target node.
    pub offset_x: i32,

    /// The offset in the Y coordinate of the mouse pointer between that event and the padding edge of the target node.
    pub offset_y: i32,

    /// The X (horizontal) coordinate (in pixels) of the mouse, relative to the left edge of the entire document. This includes any portion of the document not currently visible.
    ///
    /// Being based on the edge of the document as it is, this property takes into account any horizontal scrolling of the page. For example, if the page is scrolled such that 200 pixels of the left side of the document are scrolled out of view, and the mouse is clicked 100 pixels inward from the left edge of the view, the value returned by pageX will be 300.
    pub page_x: i32,

    /// The Y (vertical) coordinate in pixels of the event relative to the whole document.
    ///
    /// See `page_x`.
    pub page_y: i32,

    /// The X coordinate of the mouse pointer in global (screen) coordinates.
    pub screen_x: i32,

    /// The Y coordinate of the mouse pointer in global (screen) coordinates.
    pub screen_y: i32,

    /// True if the shift key was down when the mouse event was fired.
    pub shift_key: bool,
}

#[cfg(feature = "serialize")]
impl SerializedPointInteraction {
    pub fn new(
        trigger_button: Option<MouseButton>,
        held_buttons: MouseButtonSet,
        coordinates: Coordinates,
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
        Self {
            button: trigger_button
                .map_or(MouseButton::default(), |b| b)
                .into_web_code(),
            buttons: crate::input_data::encode_mouse_button_set(held_buttons),
            meta_key,
            ctrl_key,
            shift_key,
            alt_key,
            client_x,
            client_y,
            screen_x,
            screen_y,
            offset_x,
            offset_y,
            page_x,
            page_y,
        }
    }
}

#[cfg(feature = "serialize")]
impl<E: PointerInteraction> From<&E> for SerializedPointInteraction {
    fn from(data: &E) -> Self {
        let trigger_button = data.trigger_button();
        let held_buttons = data.held_buttons();
        let coordinates = data.coordinates();
        let modifiers = data.modifiers();
        Self::new(trigger_button, held_buttons, coordinates, modifiers)
    }
}

#[cfg(feature = "serialize")]
impl PointerInteraction for SerializedPointInteraction {
    fn held_buttons(&self) -> MouseButtonSet {
        crate::input_data::decode_mouse_button_set(self.buttons)
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.button))
    }
}

#[cfg(feature = "serialize")]
impl ModifiersInteraction for SerializedPointInteraction {
    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();

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
impl InteractionLocation for SerializedPointInteraction {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.client_x.into(), self.client_y.into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.screen_x.into(), self.screen_y.into())
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.page_x.into(), self.page_y.into())
    }
}

#[cfg(feature = "serialize")]
impl InteractionElementOffset for SerializedPointInteraction {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.offset_x.into(), self.offset_y.into())
    }
}
