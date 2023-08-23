use std::fmt::{Debug, Formatter};

use keyboard_types::Modifiers;

use crate::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{decode_mouse_button_set, encode_mouse_button_set, MouseButton, MouseButtonSet},
};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct PointData {
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

impl PointData {
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
            buttons: encode_mouse_button_set(held_buttons),
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

impl Debug for PointData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PointInteraction")
            .field("coordinates", &self.coordinates())
            .field("modifiers", &self.modifiers())
            .field("held_buttons", &self.held_buttons())
            .field("trigger_button", &self.trigger_button())
            .finish()
    }
}

impl PointInteraction for PointData {
    fn get_point_data(&self) -> PointData {
        *self
    }
}

pub trait PointInteraction {
    fn get_point_data(&self) -> PointData;

    fn coordinates(&self) -> Coordinates {
        let point_data = self.get_point_data();
        Coordinates::new(
            ScreenPoint::new(point_data.screen_x.into(), point_data.screen_y.into()),
            ClientPoint::new(point_data.client_x.into(), point_data.client_y.into()),
            ElementPoint::new(point_data.offset_x.into(), point_data.offset_y.into()),
            PagePoint::new(point_data.page_x.into(), point_data.page_y.into()),
        )
    }

    fn client_coordinates(&self) -> ClientPoint {
        let point_data = self.get_point_data();
        ClientPoint::new(point_data.client_x.into(), point_data.client_y.into())
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        let point_data = self.get_point_data();
        ScreenPoint::new(point_data.screen_x.into(), point_data.screen_y.into())
    }

    fn element_coordinates(&self) -> ElementPoint {
        let point_data = self.get_point_data();
        ElementPoint::new(point_data.offset_x.into(), point_data.offset_y.into())
    }

    fn page_coordinates(&self) -> PagePoint {
        let point_data = self.get_point_data();
        PagePoint::new(point_data.page_x.into(), point_data.page_y.into())
    }

    fn modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();
        let point_data = self.get_point_data();

        if point_data.alt_key {
            modifiers.insert(Modifiers::ALT);
        }
        if point_data.ctrl_key {
            modifiers.insert(Modifiers::CONTROL);
        }
        if point_data.meta_key {
            modifiers.insert(Modifiers::META);
        }
        if point_data.shift_key {
            modifiers.insert(Modifiers::SHIFT);
        }

        modifiers
    }

    fn held_buttons(&self) -> MouseButtonSet {
        decode_mouse_button_set(self.get_point_data().buttons)
    }

    fn trigger_button(&self) -> Option<MouseButton> {
        Some(MouseButton::from_web_code(self.get_point_data().button))
    }
}
