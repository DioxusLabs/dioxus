use keyboard_types::Modifiers;

use crate::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
};

// #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
// #[derive(Copy, Clone, Default, PartialEq, Eq)]
// pub struct PointData {
//     pub alt_key: bool,

//     /// The button number that was pressed (if applicable) when the mouse event was fired.
//     pub button: i16,

//     /// Indicates which buttons are pressed on the mouse (or other input device) when a mouse event is triggered.
//     ///
//     /// Each button that can be pressed is represented by a given number (see below). If more than one button is pressed, the button values are added together to produce a new number. For example, if the secondary (2) and auxiliary (4) buttons are pressed simultaneously, the value is 6 (i.e., 2 + 4).
//     ///
//     /// - 1: Primary button (usually the left button)
//     /// - 2: Secondary button (usually the right button)
//     /// - 4: Auxiliary button (usually the mouse wheel button or middle button)
//     /// - 8: 4th button (typically the "Browser Back" button)
//     /// - 16 : 5th button (typically the "Browser Forward" button)
//     pub buttons: u16,

//     /// The horizontal coordinate within the application's viewport at which the event occurred (as opposed to the coordinate within the page).
//     ///
//     /// For example, clicking on the left edge of the viewport will always result in a mouse event with a clientX value of 0, regardless of whether the page is scrolled horizontally.
//     pub client_x: i32,

//     /// The vertical coordinate within the application's viewport at which the event occurred (as opposed to the coordinate within the page).
//     ///
//     /// For example, clicking on the top edge of the viewport will always result in a mouse event with a clientY value of 0, regardless of whether the page is scrolled vertically.
//     pub client_y: i32,

//     /// True if the control key was down when the mouse event was fired.
//     pub ctrl_key: bool,

//     /// True if the meta key was down when the mouse event was fired.
//     pub meta_key: bool,

//     /// The offset in the X coordinate of the mouse pointer between that event and the padding edge of the target node.
//     pub offset_x: i32,

//     /// The offset in the Y coordinate of the mouse pointer between that event and the padding edge of the target node.
//     pub offset_y: i32,

//     /// The X (horizontal) coordinate (in pixels) of the mouse, relative to the left edge of the entire document. This includes any portion of the document not currently visible.
//     ///
//     /// Being based on the edge of the document as it is, this property takes into account any horizontal scrolling of the page. For example, if the page is scrolled such that 200 pixels of the left side of the document are scrolled out of view, and the mouse is clicked 100 pixels inward from the left edge of the view, the value returned by pageX will be 300.
//     pub page_x: i32,

//     /// The Y (vertical) coordinate in pixels of the event relative to the whole document.
//     ///
//     /// See `page_x`.
//     pub page_y: i32,

//     /// The X coordinate of the mouse pointer in global (screen) coordinates.
//     pub screen_x: i32,

//     /// The Y coordinate of the mouse pointer in global (screen) coordinates.
//     pub screen_y: i32,

//     /// True if the shift key was down when the mouse event was fired.
//     pub shift_key: bool,
// }

pub trait PointInteraction: std::any::Any {
    /// Gets the coordinates of the pointer event.
    fn coordinates(&self) -> Coordinates {
        Coordinates::new(
            self.screen_coordinates(),
            self.client_coordinates(),
            self.element_coordinates(),
            self.page_coordinates(),
        )
    }

    /// Gets the coordinates of the pointer event relative to the browser viewport.
    fn client_coordinates(&self) -> ClientPoint;

    /// Gets the coordinates of the pointer event relative to the screen.
    fn screen_coordinates(&self) -> ScreenPoint;

    /// Gets the coordinates of the pointer event relative to the target element.
    fn element_coordinates(&self) -> ElementPoint;

    /// Gets the coordinates of the pointer event relative to the page.
    fn page_coordinates(&self) -> PagePoint;

    /// Gets the modifiers of the pointer event.
    fn modifiers(&self) -> Modifiers;

    /// Gets the buttons that are currently held down.
    fn held_buttons(&self) -> MouseButtonSet;

    /// Gets the button that triggered the event.
    fn trigger_button(&self) -> Option<MouseButton>;
}

#[cfg(feature = "serialize")]
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SerializedPointInteraction {
    pub(crate) client_coordinates: ClientPoint,
    pub(crate) element_coordinates: ElementPoint,
    pub(crate) page_coordinates: PagePoint,
    pub(crate) screen_coordinates: ScreenPoint,
    pub(crate) modifiers: Modifiers,
    pub(crate) held_buttons: MouseButtonSet,
    pub(crate) trigger_button: Option<MouseButton>,
}

#[cfg(feature = "serialize")]
impl<E: PointInteraction> From<&E> for SerializedPointInteraction {
    fn from(data: &E) -> Self {
        Self {
            client_coordinates: data.client_coordinates(),
            element_coordinates: data.element_coordinates(),
            page_coordinates: data.page_coordinates(),
            screen_coordinates: data.screen_coordinates(),
            modifiers: data.modifiers(),
            held_buttons: data.held_buttons(),
            trigger_button: data.trigger_button(),
        }
    }
}

#[cfg(feature = "serialize")]
impl PointInteraction for SerializedPointInteraction {
    fn client_coordinates(&self) -> ClientPoint {
        self.client_coordinates
    }

    fn element_coordinates(&self) -> ElementPoint {
        self.element_coordinates
    }

    fn page_coordinates(&self) -> PagePoint {
        self.page_coordinates
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        self.screen_coordinates
    }

    fn coordinates(&self) -> Coordinates {
        Coordinates::new(
            self.screen_coordinates(),
            self.client_coordinates(),
            self.element_coordinates(),
            self.page_coordinates(),
        )
    }

    fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.held_buttons
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        self.trigger_button
    }
}
