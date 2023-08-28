use keyboard_types::Modifiers;

use crate::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
};

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
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone, Default)]
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
impl SerializedPointInteraction {
    /// Create a new serialized point interaction.
    pub fn new(
        client_coordinates: ClientPoint,
        element_coordinates: ElementPoint,
        page_coordinates: PagePoint,
        screen_coordinates: ScreenPoint,
        modifiers: Modifiers,
        held_buttons: MouseButtonSet,
        trigger_button: Option<MouseButton>,
    ) -> Self {
        Self {
            client_coordinates,
            element_coordinates,
            page_coordinates,
            screen_coordinates,
            modifiers,
            held_buttons,
            trigger_button,
        }
    }
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
