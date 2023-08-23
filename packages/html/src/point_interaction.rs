use keyboard_types::Modifiers;

use crate::{
    geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint},
    input_data::{MouseButton, MouseButtonSet},
};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct PointData {
    pub trigger_button: MouseButton,
    pub client_x: i32,
    pub client_y: i32,
    pub screen_x: i32,
    pub screen_y: i32,
    pub element_x: i32,
    pub element_y: i32,
    pub page_x: i32,
    pub page_y: i32,
    pub held_buttons: MouseButtonSet,
    pub modifiers: Modifiers,
}

impl PointData {
    pub fn new(
        trigger_button: Option<MouseButton>,
        coordinates: Coordinates,
        held_buttons: MouseButtonSet,
        modifiers: Modifiers,
    ) -> Self {
        let [client_x, client_y]: [i32; 2] = coordinates.client().cast().into();
        let [element_x, element_y]: [i32; 2] = coordinates.element().cast().into();
        let [page_x, page_y]: [i32; 2] = coordinates.page().cast().into();
        let [screen_x, screen_y]: [i32; 2] = coordinates.screen().cast().into();
        Self {
            trigger_button: trigger_button.map_or(MouseButton::default(), |b| b),
            client_x,
            client_y,
            screen_x,
            screen_y,
            element_x,
            element_y,
            page_x,
            page_y,
            held_buttons,
            modifiers,
        }
    }
}

pub trait PointInteraction {
    fn get_point_data(&self) -> PointData;

    fn coordinates(&self) -> Coordinates {
        let point_data = self.get_point_data();
        Coordinates::new(
            ScreenPoint::new(point_data.screen_x.into(), point_data.screen_y.into()),
            ClientPoint::new(point_data.client_x.into(), point_data.client_y.into()),
            ElementPoint::new(point_data.element_x.into(), point_data.element_y.into()),
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
        ElementPoint::new(point_data.element_x.into(), point_data.element_y.into())
    }

    fn page_coordinates(&self) -> PagePoint {
        let point_data = self.get_point_data();
        PagePoint::new(point_data.page_x.into(), point_data.page_y.into())
    }

    fn modifiers(&self) -> Modifiers {
        self.get_point_data().modifiers
    }

    fn held_buttons(&self) -> MouseButtonSet {
        self.get_point_data().held_buttons
    }

    fn trigger_button(&self) -> MouseButton {
        self.get_point_data().trigger_button
    }
}
