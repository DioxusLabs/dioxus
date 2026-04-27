use blitz_dom::{BaseDocument, Node};
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, BlitzScrollEvent, BlitzWheelDelta,
    BlitzWheelEvent, MouseEventButton,
};
use dioxus_html::{
    AnimationData, CancelData, ClipboardData, CompositionData, DragData, FocusData, FormData,
    FormValue, HasFileData, HasFocusData, HasFormData, HasKeyboardData, HasMouseData,
    HasPointerData, HasScrollData, HasWheelData, HtmlEventConverter, ImageData, KeyboardData,
    MediaData, MountedData, MountedError, MountedResult, MouseData, PlatformEventData, PointerData,
    RenderedElementBacking, ResizeData, ScrollBehavior, ScrollData, ScrollToOptions, SelectionData,
    ToggleData, TouchData, TransitionData, VisibleData, WheelData,
    geometry::{
        ClientPoint, ElementPoint, PagePoint, PixelsRect, PixelsSize, PixelsVector2D, ScreenPoint,
        WheelDelta,
        euclid::{Point2D, Size2D, Vector3D},
    },
    input_data::{MouseButton, MouseButtonSet},
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
};
use keyboard_types::{Code, Key, Location, Modifiers};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    fmt::Display,
    future::Future,
    pin::Pin,
    rc::Rc,
};

use crate::NodeId;

pub struct NativeConverter {}

impl HtmlEventConverter for NativeConverter {
    fn convert_cancel_data(&self, _event: &PlatformEventData) -> CancelData {
        unimplemented!("todo: convert_cancel_data in dioxus-native. requires support in blitz")
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        event.downcast::<NativeFormData>().unwrap().clone().into()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event
            .downcast::<NativePointerData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        event
            .downcast::<BlitzKeyboardData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_focus_data(&self, _event: &PlatformEventData) -> FocusData {
        NativeFocusData {}.into()
    }

    fn convert_animation_data(&self, _event: &PlatformEventData) -> AnimationData {
        unimplemented!("todo: convert_animation_data in dioxus-native. requires support in blitz")
    }

    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> ClipboardData {
        unimplemented!("todo: convert_clipboard_data in dioxus-native. requires support in blitz")
    }

    fn convert_composition_data(&self, _event: &PlatformEventData) -> CompositionData {
        unimplemented!("todo: convert_composition_data in dioxus-native. requires support in blitz")
    }

    fn convert_drag_data(&self, _event: &PlatformEventData) -> DragData {
        unimplemented!("todo: convert_drag_data in dioxus-native. requires support in blitz")
    }

    fn convert_image_data(&self, _event: &PlatformEventData) -> ImageData {
        unimplemented!("todo: convert_image_data in dioxus-native. requires support in blitz")
    }

    fn convert_media_data(&self, _event: &PlatformEventData) -> MediaData {
        unimplemented!("todo: convert_media_data in dioxus-native. requires support in blitz")
    }

    fn convert_mounted_data(&self, event: &PlatformEventData) -> MountedData {
        event.downcast::<NodeHandle>().unwrap().clone().into()
    }

    fn convert_pointer_data(&self, event: &PlatformEventData) -> PointerData {
        event
            .downcast::<NativePointerData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_scroll_data(&self, event: &PlatformEventData) -> ScrollData {
        event.downcast::<NativeScrollData>().unwrap().clone().into()
    }

    fn convert_selection_data(&self, _event: &PlatformEventData) -> SelectionData {
        unimplemented!("todo: convert_selection_data in dioxus-native. requires support in blitz")
    }

    fn convert_toggle_data(&self, _event: &PlatformEventData) -> ToggleData {
        unimplemented!("todo: convert_toggle_data in dioxus-native. requires support in blitz")
    }

    fn convert_touch_data(&self, _event: &PlatformEventData) -> TouchData {
        unimplemented!("todo: convert_touch_data in dioxus-native. requires support in blitz")
    }

    fn convert_transition_data(&self, _event: &PlatformEventData) -> TransitionData {
        unimplemented!("todo: convert_transition_data in dioxus-native. requires support in blitz")
    }

    fn convert_wheel_data(&self, event: &PlatformEventData) -> WheelData {
        event.downcast::<NativeWheelData>().unwrap().clone().into()
    }

    fn convert_resize_data(&self, _event: &PlatformEventData) -> ResizeData {
        unimplemented!("todo: convert_resize_data in dioxus-native. requires support in blitz")
    }

    fn convert_visible_data(&self, _event: &PlatformEventData) -> VisibleData {
        unimplemented!("todo: convert_visible_data in dioxus-native. requires support in blitz")
    }
}

#[derive(Clone)]
pub struct NodeHandle {
    pub(crate) doc: Rc<RefCell<BaseDocument>>,
    pub(crate) node_id: NodeId,
}

impl NodeHandle {
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn doc(&self) -> Ref<'_, BaseDocument> {
        self.doc.borrow()
    }

    pub fn doc_mut(&self) -> RefMut<'_, BaseDocument> {
        self.doc.borrow_mut()
    }

    pub fn node(&self) -> Ref<'_, Node> {
        Ref::map(self.doc.borrow(), |doc| {
            doc.get_node(self.node_id)
                .expect("Node does not exist in the Document")
        })
    }

    pub fn node_mut(&self) -> RefMut<'_, Node> {
        RefMut::map(self.doc.borrow_mut(), |doc| {
            doc.get_node_mut(self.node_id)
                .expect("Node does not exist in the Document")
        })
    }

    fn node_not_exist_err<T>(&self) -> Pin<Box<dyn Future<Output = MountedResult<T>>>> {
        let node_id = self.node_id;
        let err = MountedError::OperationFailed(Box::new(NodeNotExistErr(node_id)));
        Box::pin(async move { Err(err) })
    }
}

#[derive(Debug)]
struct NodeNotExistErr(NodeId);
impl Display for NodeNotExistErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The node {} does not exist", self.0)
    }
}
impl std::error::Error for NodeNotExistErr {}

impl RenderedElementBacking for NodeHandle {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_scroll_offset(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsVector2D>>>> {
        let scroll_offset = self.node().scroll_offset;
        Box::pin(async move { Ok(PixelsVector2D::new(scroll_offset.x, scroll_offset.y)) })
    }

    fn get_scroll_size(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsSize>>>> {
        let node = self.node();
        let scroll_width = node.final_layout.scroll_width() as f64;
        let scroll_height = node.final_layout.scroll_height() as f64;
        Box::pin(async move { Ok(PixelsSize::new(scroll_width, scroll_height)) })
    }

    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        let Some(bounding_rect) = self.doc_mut().get_client_bounding_rect(self.node_id) else {
            return self.node_not_exist_err();
        };
        let pixels_rect = PixelsRect::new(
            Point2D::new(bounding_rect.x, bounding_rect.y),
            Size2D::new(bounding_rect.width, bounding_rect.height),
        );
        Box::pin(async move { Ok(pixels_rect) })
    }

    fn scroll_to(
        &self,
        _options: ScrollToOptions,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    fn scroll(
        &self,
        _coordinates: PixelsVector2D,
        _behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        Box::pin(async { Err(MountedError::NotSupported) })
    }

    fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let mut doc = self.doc_mut();
        if focus {
            // TODO: queue focus events somehow
            doc.set_focus_to(self.node_id);
        } else if doc.get_focussed_node_id() == Some(self.node_id) {
            // Q: Should this only clear focus if the node is focussed?
            // TODO: queue blur events somehow
            doc.clear_focus();
        }

        Box::pin(async { Ok(()) })
    }
}

#[derive(Clone, Debug)]
pub struct NativeFormData {
    pub value: String,
    pub values: Vec<(String, FormValue)>,
}

impl HasFormData for NativeFormData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn value(&self) -> String {
        self.value.clone()
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        self.values.clone()
    }
    fn valid(&self) -> bool {
        // todo: actually implement validation here.
        true
    }
}

impl HasFileData for NativeFormData {
    fn files(&self) -> Vec<dioxus_html::FileData> {
        vec![]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BlitzKeyboardData(pub(crate) BlitzKeyEvent);

impl ModifiersInteraction for BlitzKeyboardData {
    fn modifiers(&self) -> Modifiers {
        self.0.modifiers
    }
}

impl HasKeyboardData for BlitzKeyboardData {
    fn key(&self) -> Key {
        self.0.key.clone()
    }

    fn code(&self) -> Code {
        self.0.code
    }

    fn location(&self) -> Location {
        self.0.location
    }

    fn is_auto_repeating(&self) -> bool {
        self.0.is_auto_repeating
    }

    fn is_composing(&self) -> bool {
        self.0.is_composing
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativePointerData(pub(crate) BlitzPointerEvent);

impl InteractionLocation for NativePointerData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.client_x() as f64, self.0.client_y() as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0.screen_x() as f64, self.0.screen_y() as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0.page_x() as f64, self.0.page_y() as f64)
    }
}

impl InteractionElementOffset for NativePointerData {
    fn element_coordinates(&self) -> ElementPoint {
        // TODO: implement element point
        ElementPoint::new(0.0, 0.0)
    }
}

impl ModifiersInteraction for NativePointerData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

impl PointerInteraction for NativePointerData {
    fn trigger_button(&self) -> Option<MouseButton> {
        Some(match self.0.button {
            MouseEventButton::Main => MouseButton::Primary,
            MouseEventButton::Auxiliary => MouseButton::Auxiliary,
            MouseEventButton::Secondary => MouseButton::Secondary,
            MouseEventButton::Fourth => MouseButton::Fourth,
            MouseEventButton::Fifth => MouseButton::Fifth,
        })
    }

    fn held_buttons(&self) -> MouseButtonSet {
        dioxus_html::input_data::decode_mouse_button_set(self.0.buttons.bits() as u16)
    }
}
impl HasMouseData for NativePointerData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl HasPointerData for NativePointerData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn is_primary(&self) -> bool {
        self.0.is_primary
    }

    fn pointer_id(&self) -> i32 {
        match self.0.id {
            BlitzPointerId::Mouse => 0,
            BlitzPointerId::Pen => 0,
            BlitzPointerId::Finger(id) => id as i32,
        }
    }

    fn pointer_type(&self) -> String {
        match self.0.id {
            BlitzPointerId::Mouse => String::from("mouse"),
            BlitzPointerId::Pen => String::from("pen"),
            BlitzPointerId::Finger(_) => String::from("touch"),
        }
    }

    fn pressure(&self) -> f32 {
        self.0.details.pressure as f32
    }
    fn tangential_pressure(&self) -> f32 {
        self.0.details.tangential_pressure
    }
    fn tilt_x(&self) -> i32 {
        self.0.details.tilt_x as i32
    }
    fn tilt_y(&self) -> i32 {
        self.0.details.tilt_y as i32
    }
    fn twist(&self) -> i32 {
        self.0.details.twist as i32
    }

    // TODO: implement these fields with real values
    fn width(&self) -> f64 {
        1.0
    }
    fn height(&self) -> f64 {
        1.0
    }
}

#[derive(Clone)]
pub struct NativeFocusData;
impl HasFocusData for NativeFocusData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativeScrollData(pub(crate) BlitzScrollEvent);
impl HasScrollData for NativeScrollData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn scroll_top(&self) -> f64 {
        self.0.scroll_top
    }

    fn scroll_left(&self) -> f64 {
        self.0.scroll_left
    }

    fn scroll_width(&self) -> i32 {
        self.0.scroll_width
    }

    fn scroll_height(&self) -> i32 {
        self.0.scroll_height
    }

    fn client_width(&self) -> i32 {
        self.0.client_width
    }

    fn client_height(&self) -> i32 {
        self.0.client_height
    }
}

#[derive(Clone)]
pub struct NativeWheelData(pub(crate) BlitzWheelEvent);
impl HasWheelData for NativeWheelData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn delta(&self) -> WheelDelta {
        match self.0.delta {
            BlitzWheelDelta::Lines(x, y) => {
                dioxus_html::geometry::WheelDelta::Lines(Vector3D::new(x, y, 0.0))
            }
            BlitzWheelDelta::Pixels(x, y) => {
                dioxus_html::geometry::WheelDelta::Pixels(Vector3D::new(x, y, 0.0))
            }
        }
    }
}

impl HasMouseData for NativeWheelData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl PointerInteraction for NativeWheelData {
    fn trigger_button(&self) -> Option<MouseButton> {
        None
    }

    fn held_buttons(&self) -> MouseButtonSet {
        dioxus_html::input_data::decode_mouse_button_set(self.0.buttons.bits() as u16)
    }
}

impl ModifiersInteraction for NativeWheelData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

impl InteractionElementOffset for NativeWheelData {
    fn element_coordinates(&self) -> ElementPoint {
        // TODO: implement element point
        ElementPoint::new(0.0, 0.0)
    }
}

impl InteractionLocation for NativeWheelData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.client_x() as f64, self.0.client_y() as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0.screen_x() as f64, self.0.screen_y() as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0.page_x() as f64, self.0.page_y() as f64)
    }
}
